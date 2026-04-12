//! Extension traits that add `.to_sql()` rendering to DOL builders.
//!
//! These traits bridge `dol-builder` (which produces IR) with `dol-sql`
//! (which renders IR to SQL strings), keeping the dependency arrow from
//! `dol-sql` → `dol-builder` rather than the reverse.

use crate::dialect::{self, Dialect};
use crate::render;
use dol_builder::control::{GrantBuilder, RevokeBuilder};
use dol_builder::definition::{
    AlterModelBuilder, CreateFromMeta, DefineIndexBuilder, DefineModelBuilder, DropIndexBuilder,
    DropModelBuilder,
};
use dol_builder::mutation::{
    InsertBuilder, InsertSelectBuilder, RemoveBuilder, UpdateBuilder, UpsertBuilder,
};
use dol_builder::query::GetBuilder;
use dol_builder::transaction::TransactionBuilder;
use dol_expr::{Expr, OrderByExpr};
use dol_ir::query::SetOpKind;
use dol_ir::transaction::TransactionIR;
use dol_ir::OffsetLimit;

// ===========================================================================
// ToSql — generic SQL rendering trait
// ===========================================================================

/// Extension trait adding `.to_sql()` to DOL builders.
pub trait ToSql {
    /// Render to a SQL string. Pass `None` for the global default dialect.
    fn to_sql(&self, dialect: Option<&Dialect>) -> String;
}

// ── GetBuilder ──────────────────────────────────────────────────────────

impl ToSql for GetBuilder<'_> {
    fn to_sql(&self, dialect: Option<&Dialect>) -> String {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.clone().build();
        render::render_query_ir(&ir, dialect)
            .expect("Query rendering should not fail")
            .sql
    }
}

// ── InsertBuilder ───────────────────────────────────────────────────────

impl ToSql for InsertBuilder<'_> {
    fn to_sql(&self, dialect: Option<&Dialect>) -> String {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_insert_ir(&ir, dialect)
            .expect("Insert rendering should not fail")
            .sql
    }
}

// ── InsertSelectBuilder ─────────────────────────────────────────────────

impl ToSql for InsertSelectBuilder<'_> {
    fn to_sql(&self, dialect: Option<&Dialect>) -> String {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_insert_select_ir(&ir, dialect)
            .expect("InsertSelect rendering should not fail")
            .sql
    }
}

// ── UpdateBuilder ───────────────────────────────────────────────────────

impl ToSql for UpdateBuilder<'_> {
    fn to_sql(&self, dialect: Option<&Dialect>) -> String {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_update_ir(&ir, dialect)
            .expect("Update rendering should not fail")
            .sql
    }
}

// ── RemoveBuilder ───────────────────────────────────────────────────────

impl ToSql for RemoveBuilder<'_> {
    fn to_sql(&self, dialect: Option<&Dialect>) -> String {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_remove_ir(&ir, dialect)
            .expect("Remove rendering should not fail")
            .sql
    }
}

// ── UpsertBuilder ───────────────────────────────────────────────────────

impl ToSql for UpsertBuilder<'_> {
    fn to_sql(&self, dialect: Option<&Dialect>) -> String {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_upsert_ir(&ir, dialect)
            .expect("Upsert rendering should not fail")
            .sql
    }
}

// ── CreateFromMeta ──────────────────────────────────────────────────────

impl ToSql for CreateFromMeta<'_> {
    fn to_sql(&self, dialect: Option<&Dialect>) -> String {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let model = self.get_model();
        let mut sql = String::from("CREATE TABLE ");
        if self.has_if_not_exists() {
            sql.push_str("IF NOT EXISTS ");
        }
        sql.push_str(&model.qualified_name());
        sql.push_str(" (\n");

        let mut parts = Vec::new();

        // Field definitions
        for field in model.fields {
            parts.push(format!("  {}", render::render_field_def(field, dialect)));
        }

        // Primary key constraint (derived from fields marked as PK)
        let pk_fields: Vec<&str> = model
            .fields
            .iter()
            .filter(|f| f.primary_key)
            .map(|f| f.name)
            .collect();
        if !pk_fields.is_empty() {
            parts.push(format!("  PRIMARY KEY ({})", pk_fields.join(", ")));
        }

        // Model-level constraints
        for constraint in model.constraints {
            parts.push(format!("  {}", render::render_model_constraint(constraint)));
        }

        sql.push_str(&parts.join(",\n"));
        sql.push_str("\n)");
        sql
    }
}

// ── DefineModelBuilder ──────────────────────────────────────────────────

impl ToSql for DefineModelBuilder {
    fn to_sql(&self, dialect: Option<&Dialect>) -> String {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_define_model_ir(&ir, dialect)
            .expect("DefineModel rendering should not fail")
            .sql
    }
}

// ── AlterModelBuilder ───────────────────────────────────────────────────

impl ToSql for AlterModelBuilder<'_> {
    fn to_sql(&self, dialect: Option<&Dialect>) -> String {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_alter_model_ir(&ir, dialect)
            .expect("AlterModel rendering should not fail")
            .sql
    }
}

// ── DropModelBuilder ────────────────────────────────────────────────────

impl ToSql for DropModelBuilder<'_> {
    fn to_sql(&self, dialect: Option<&Dialect>) -> String {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_drop_model_ir(&ir, dialect)
            .expect("DropModel rendering should not fail")
            .sql
    }
}

// ── DefineIndexBuilder ──────────────────────────────────────────────────

impl ToSql for DefineIndexBuilder {
    fn to_sql(&self, dialect: Option<&Dialect>) -> String {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_define_index_ir(&ir, dialect)
            .expect("DefineIndex rendering should not fail")
            .sql
    }
}

// ── DropIndexBuilder ────────────────────────────────────────────────────

impl ToSql for DropIndexBuilder {
    fn to_sql(&self, dialect: Option<&Dialect>) -> String {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_drop_index_ir(&ir, dialect)
            .expect("DropIndex rendering should not fail")
            .sql
    }
}

// ── GrantBuilder ────────────────────────────────────────────────────────

impl ToSql for GrantBuilder {
    fn to_sql(&self, dialect: Option<&Dialect>) -> String {
        let _dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_grant_ir(&ir)
            .expect("Grant rendering should not fail")
            .sql
    }
}

// ── RevokeBuilder ───────────────────────────────────────────────────────

impl ToSql for RevokeBuilder {
    fn to_sql(&self, dialect: Option<&Dialect>) -> String {
        let _dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_revoke_ir(&ir)
            .expect("Revoke rendering should not fail")
            .sql
    }
}

// ===========================================================================
// TransactionBuilder SQL extension
// ===========================================================================

/// Extension trait adding `.to_sql()` to [`TransactionBuilder`].
pub trait TransactionSqlExt {
    /// Render a TransactionIR to SQL.
    fn to_sql(ir: &TransactionIR) -> String;
}

impl TransactionSqlExt for TransactionBuilder {
    fn to_sql(ir: &TransactionIR) -> String {
        render::render_transaction_ir(ir)
            .expect("Transaction rendering should not fail")
            .sql
    }
}

// ===========================================================================
// CompoundSelectBuilder — compound queries (moved from dol-builder)
// ===========================================================================

/// A compound SELECT composed of multiple queries joined by set operations.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .to_sql() is called"]
pub struct CompoundSelectBuilder {
    base: String,
    parts: Vec<(SetOpKind, String)>,
    order_by: Vec<OrderByExpr>,
    has_offset: bool,
    has_limit: bool,
}

impl CompoundSelectBuilder {
    pub fn new(base_sql: String) -> Self {
        Self {
            base: base_sql,
            parts: Vec::new(),
            order_by: Vec::new(),
            has_offset: false,
            has_limit: false,
        }
    }

    pub fn union(mut self, query: String) -> Self {
        self.parts.push((SetOpKind::Union, query));
        self
    }

    pub fn union_all(mut self, query: String) -> Self {
        self.parts.push((SetOpKind::UnionAll, query));
        self
    }

    pub fn intersect(mut self, query: String) -> Self {
        self.parts.push((SetOpKind::Intersect, query));
        self
    }

    pub fn intersect_all(mut self, query: String) -> Self {
        self.parts.push((SetOpKind::IntersectAll, query));
        self
    }

    pub fn except(mut self, query: String) -> Self {
        self.parts.push((SetOpKind::Except, query));
        self
    }

    pub fn except_all(mut self, query: String) -> Self {
        self.parts.push((SetOpKind::ExceptAll, query));
        self
    }

    pub fn op(mut self, kind: SetOpKind, query: String) -> Self {
        self.parts.push((kind, query));
        self
    }

    pub fn order_by(mut self, exprs: impl IntoIterator<Item = OrderByExpr>) -> Self {
        self.order_by = exprs.into_iter().collect();
        self
    }

    pub fn offset(mut self) -> Self {
        self.has_offset = true;
        self
    }

    pub fn limit(mut self) -> Self {
        self.has_limit = true;
        self
    }
}

impl ToSql for CompoundSelectBuilder {
    fn to_sql(&self, dialect: Option<&Dialect>) -> String {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let mut counter = dialect.param_counter();
        let mut sql = self.base.clone();

        for (kind, query) in &self.parts {
            let kind_str = match kind {
                SetOpKind::Union => "UNION",
                SetOpKind::UnionAll => "UNION ALL",
                SetOpKind::Intersect => "INTERSECT",
                SetOpKind::IntersectAll => "INTERSECT ALL",
                SetOpKind::Except => "EXCEPT",
                SetOpKind::ExceptAll => "EXCEPT ALL",
            };
            sql.push_str(&format!(" {} {}", kind_str, query));
        }

        if !self.order_by.is_empty() {
            sql.push_str(&render::render_order_by_exprs(
                &self.order_by,
                &mut counter,
                dialect,
            ));
        }

        if self.has_offset || self.has_limit {
            let offset_ol = if self.has_offset {
                Some(OffsetLimit::Param)
            } else {
                None
            };
            let limit_ol = if self.has_limit {
                Some(OffsetLimit::Param)
            } else {
                None
            };
            sql.push_str(&render::render_pagination(
                &offset_ol,
                &limit_ol,
                &mut counter,
                dialect,
            ));
        }

        sql
    }
}

// ===========================================================================
// GetBuilder compound/subquery extensions
// ===========================================================================

/// Extension trait adding compound query and subquery methods to [`GetBuilder`].
pub trait GetBuilderSqlExt {
    /// Start a UNION compound query with this builder's SQL as the base.
    fn union(self, other: GetBuilder<'_>) -> CompoundSelectBuilder;
    /// Start a UNION ALL compound query.
    fn union_all(self, other: GetBuilder<'_>) -> CompoundSelectBuilder;
    /// Start an INTERSECT compound query.
    fn intersect(self, other: GetBuilder<'_>) -> CompoundSelectBuilder;
    /// Start an EXCEPT compound query.
    fn except(self, other: GetBuilder<'_>) -> CompoundSelectBuilder;
    /// Render this query as a scalar subquery expression `(SELECT ...)`.
    fn as_scalar(&self) -> Expr;
    /// Render this query as a scalar subquery expression using a specific dialect.
    fn as_scalar_with(&self, dialect: &Dialect) -> Expr;
}

impl GetBuilderSqlExt for GetBuilder<'_> {
    fn union(self, other: GetBuilder<'_>) -> CompoundSelectBuilder {
        let base = ToSql::to_sql(&self, None);
        let rhs = ToSql::to_sql(&other, None);
        CompoundSelectBuilder::new(base).union(rhs)
    }

    fn union_all(self, other: GetBuilder<'_>) -> CompoundSelectBuilder {
        let base = ToSql::to_sql(&self, None);
        let rhs = ToSql::to_sql(&other, None);
        CompoundSelectBuilder::new(base).union_all(rhs)
    }

    fn intersect(self, other: GetBuilder<'_>) -> CompoundSelectBuilder {
        let base = ToSql::to_sql(&self, None);
        let rhs = ToSql::to_sql(&other, None);
        CompoundSelectBuilder::new(base).intersect(rhs)
    }

    fn except(self, other: GetBuilder<'_>) -> CompoundSelectBuilder {
        let base = ToSql::to_sql(&self, None);
        let rhs = ToSql::to_sql(&other, None);
        CompoundSelectBuilder::new(base).except(rhs)
    }

    fn as_scalar(&self) -> Expr {
        Expr::Subquery(ToSql::to_sql(self, None))
    }

    fn as_scalar_with(&self, dialect: &Dialect) -> Expr {
        Expr::Subquery(ToSql::to_sql(self, Some(dialect)))
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use dol_builder::ModelBuilderExt;
    use dol_ir::definition::FieldDef;
    use dol_model::{Field, FieldType, Model};

    fn pg() -> Dialect {
        Dialect::postgres()
    }

    static TEST_MODEL: Model = Model::new(
        "users",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("tenant_id", FieldType::Uuid),
            Field::new("email", FieldType::Text).unique(),
            Field::new("status", FieldType::Text).default("'active'"),
            Field::new("created_at", FieldType::Timestamp).default("NOW()"),
        ],
    );

    static NS_MODEL: Model =
        Model::new("users", &[Field::new("id", FieldType::Uuid).primary_key()])
            .with_namespace("auth");

    // -- GetBuilder --

    #[test]
    fn get_builder_to_sql() {
        let sql = TEST_MODEL
            .get()
            .all_columns()
            .where_eq("status")
            .order_by_desc("email")
            .limit()
            .to_sql(Some(&pg()));
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("FROM users"));
    }

    // -- InsertBuilder --

    #[test]
    fn insert_all_columns() {
        let sql = TEST_MODEL.insert().all_columns().to_sql(Some(&pg()));
        assert!(sql.contains("INSERT INTO users (id, tenant_id, email, status, created_at)"));
        assert!(sql.contains("VALUES ($1, $2, $3, $4, $5)"));
    }

    #[test]
    fn insert_specific_columns() {
        let sql = TEST_MODEL
            .insert()
            .columns(&["id", "email"])
            .to_sql(Some(&pg()));
        assert!(sql.contains("(id, email)"));
        assert!(sql.contains("($1, $2)"));
    }

    #[test]
    fn insert_multiple_rows() {
        let sql = TEST_MODEL
            .insert()
            .columns(&["id", "email"])
            .rows(3)
            .to_sql(Some(&pg()));
        assert!(sql.contains("($1, $2), ($3, $4), ($5, $6)"));
    }

    #[test]
    fn insert_returning_all() {
        let sql = TEST_MODEL
            .insert()
            .columns(&["id"])
            .returning_all()
            .to_sql(None);
        assert!(sql.contains("RETURNING *"));
    }

    #[test]
    fn insert_with_namespace() {
        let sql = NS_MODEL.insert().columns(&["id"]).to_sql(None);
        assert!(sql.contains("INSERT INTO auth.users"));
    }

    // -- UpdateBuilder --

    #[test]
    fn update_set_and_where() {
        let sql = TEST_MODEL
            .update()
            .set("email")
            .set("status")
            .where_eq("id")
            .to_sql(Some(&pg()));
        assert!(sql.contains("UPDATE users SET"));
        assert!(sql.contains("email = $1"));
        assert!(sql.contains("status = $2"));
        assert!(sql.contains("WHERE id = $3"));
    }

    // -- RemoveBuilder --

    #[test]
    fn remove_basic() {
        let sql = TEST_MODEL.remove().where_eq("id").to_sql(Some(&pg()));
        assert!(sql.contains("DELETE FROM users"));
        assert!(sql.contains("WHERE id = $1"));
    }

    // -- UpsertBuilder --

    #[test]
    fn upsert_basic() {
        let sql = TEST_MODEL
            .upsert()
            .columns(&["id", "email", "status"])
            .on_conflict(&["id"])
            .do_update(&["email", "status"])
            .to_sql(Some(&pg()));
        assert!(sql.contains("INSERT INTO users (id, email, status)"));
        assert!(sql.contains("ON CONFLICT (id)"));
        assert!(sql.contains("DO UPDATE SET"));
    }

    // -- CreateFromMeta --

    #[test]
    fn create_from_meta_basic() {
        let sql = TEST_MODEL.create().to_sql(Some(&pg()));
        assert!(sql.starts_with("CREATE TABLE users ("));
        assert!(sql.contains("id UUID NOT NULL"));
        assert!(sql.contains("email TEXT NOT NULL UNIQUE"));
        assert!(sql.contains("status TEXT NOT NULL DEFAULT 'active'"));
        assert!(sql.contains("PRIMARY KEY (id)"));
    }

    #[test]
    fn create_from_meta_if_not_exists() {
        let sql = TEST_MODEL.create().if_not_exists().to_sql(None);
        assert!(sql.starts_with("CREATE TABLE IF NOT EXISTS users ("));
    }

    // -- DefineModelBuilder --

    #[test]
    fn define_model_builder_basic() {
        use dol_builder::ModelDefineExt;
        let sql = Model::define("sessions")
            .field(FieldDef::new("id", FieldType::Uuid).primary_key())
            .field(FieldDef::new("user_id", FieldType::Uuid))
            .if_not_exists()
            .to_sql(Some(&pg()));
        assert!(sql.starts_with("CREATE TABLE IF NOT EXISTS sessions ("));
        assert!(sql.contains("id UUID NOT NULL"));
    }

    // -- AlterModelBuilder --

    #[test]
    fn alter_model_add_and_drop() {
        let sql = TEST_MODEL
            .alter()
            .add_field(FieldDef::new("phone", FieldType::Text).nullable())
            .drop_field("legacy")
            .to_sql(Some(&pg()));
        assert!(sql.contains("ADD COLUMN phone TEXT"));
        assert!(sql.contains("DROP COLUMN legacy"));
    }

    // -- DropModelBuilder --

    #[test]
    fn drop_model_basic() {
        let sql = TEST_MODEL.drop_model().to_sql(None);
        assert_eq!(sql, "DROP TABLE users");
    }

    #[test]
    fn drop_model_if_exists_cascade() {
        let sql = TEST_MODEL.drop_model().if_exists().cascade().to_sql(None);
        assert_eq!(sql, "DROP TABLE IF EXISTS users CASCADE");
    }

    // -- DefineIndexBuilder --

    #[test]
    fn define_index_basic() {
        use dol_builder::DefineIndexBuilder;
        let sql = DefineIndexBuilder::new("idx_users_email")
            .on("users")
            .columns(&["tenant_id", "email"])
            .unique()
            .to_sql(None);
        assert!(sql.contains("CREATE UNIQUE INDEX"));
        assert!(sql.contains("ON users"));
    }

    // -- DropIndexBuilder --

    #[test]
    fn drop_index_basic() {
        use dol_builder::DropIndexBuilder;
        let sql = DropIndexBuilder::new("idx_users_email").to_sql(None);
        assert_eq!(sql, "DROP INDEX idx_users_email");
    }

    // -- GrantBuilder --

    #[test]
    fn grant_basic() {
        use dol_builder::control::{GrantBuilder, Privilege};
        let sql = GrantBuilder::new(Privilege::Select)
            .on("users")
            .to("app_reader")
            .to_sql(None);
        assert_eq!(sql, "GRANT SELECT ON users TO app_reader");
    }

    // -- RevokeBuilder --

    #[test]
    fn revoke_basic() {
        use dol_builder::control::{Privilege, RevokeBuilder};
        let sql = RevokeBuilder::new(Privilege::Insert)
            .on("users")
            .from("app_reader")
            .to_sql(None);
        assert_eq!(sql, "REVOKE INSERT ON users FROM app_reader");
    }
}
