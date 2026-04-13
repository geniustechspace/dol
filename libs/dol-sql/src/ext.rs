//! Extension traits that add `.to_sql()` and `.try_to_sql()` rendering to DOL builders.
//!
//! These traits bridge `dol-builder` (which produces IR) with `dol-sql`
//! (which renders IR to SQL strings), keeping the dependency arrow from
//! `dol-sql` → `dol-builder` rather than the reverse.
//!
//! # Error handling
//!
//! - [`TryToSql`] returns `Result<String, BackendError>` for fallible rendering.
//! - [`ToSql`] is a convenience wrapper that falls back to an error placeholder
//!   instead of panicking.

use crate::dialect::{self, Dialect};
use crate::render;
use dol_core::builder::control::{GrantBuilder, RevokeBuilder};
use dol_core::builder::definition::{
    AlterModelBuilder, CreateFromMeta, DefineIndexBuilder, DefineModelBuilder, DropIndexBuilder,
    DropModelBuilder,
};
use dol_core::builder::mutation::{
    InsertBuilder, InsertSelectBuilder, RemoveBuilder, UpdateBuilder, UpsertBuilder,
};
use dol_core::builder::query::GetBuilder;
use dol_core::builder::transaction::TransactionBuilder;
use dol_core::expr::{Expr, OrderByExpr};
use dol_core::ir::BackendError;
use dol_core::ir::OffsetLimit;
use dol_core::ir::query::{CompoundQueryIR, QueryIR, SetOpKind};
use dol_core::ir::transaction::TransactionIR;

// ===========================================================================
// TryToSql — fallible SQL rendering trait
// ===========================================================================

/// Extension trait adding fallible `.try_to_sql()` to DOL builders.
///
/// Returns `Result<String, BackendError>` instead of panicking on render errors.
pub trait TryToSql {
    /// Render to a SQL string, returning an error on failure.
    /// Pass `None` for the global default dialect.
    fn try_to_sql(&self, dialect: Option<&Dialect>) -> Result<String, BackendError>;
}

// ===========================================================================
// ToSql — convenience (infallible) SQL rendering trait
// ===========================================================================

/// Extension trait adding `.to_sql()` to DOL builders.
///
/// This is a convenience wrapper around [`TryToSql`]. On rendering failure,
/// it returns an error placeholder string (`"/* SQL render error: ... */"`)
/// instead of panicking.
///
/// For production code that needs to handle errors, prefer [`TryToSql`].
pub trait ToSql {
    /// Render to a SQL string. Pass `None` for the global default dialect.
    fn to_sql(&self, dialect: Option<&Dialect>) -> String;
}

/// Sanitize an error message for embedding inside a SQL block comment.
///
/// Replaces `*/` sequences so the comment cannot be terminated early,
/// which would otherwise risk turning the remainder into executable SQL.
fn sanitize_for_sql_comment(msg: &str) -> String {
    msg.replace("*/", "* /")
}

/// Blanket implementation: any type implementing `TryToSql` also gets `ToSql`.
impl<T: TryToSql> ToSql for T {
    fn to_sql(&self, dialect: Option<&Dialect>) -> String {
        match self.try_to_sql(dialect) {
            Ok(sql) => sql,
            Err(e) => format!(
                "/* SQL render error: {} */",
                sanitize_for_sql_comment(&e.to_string())
            ),
        }
    }
}

// ── GetBuilder ──────────────────────────────────────────────────────────

impl TryToSql for GetBuilder<'_> {
    fn try_to_sql(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.clone().build();
        render::render_query_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── InsertBuilder ───────────────────────────────────────────────────────

impl TryToSql for InsertBuilder<'_> {
    fn try_to_sql(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_insert_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── InsertSelectBuilder ─────────────────────────────────────────────────

impl TryToSql for InsertSelectBuilder<'_> {
    fn try_to_sql(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_insert_select_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── UpdateBuilder ───────────────────────────────────────────────────────

impl TryToSql for UpdateBuilder<'_> {
    fn try_to_sql(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_update_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── RemoveBuilder ───────────────────────────────────────────────────────

impl TryToSql for RemoveBuilder<'_> {
    fn try_to_sql(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_remove_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── UpsertBuilder ───────────────────────────────────────────────────────

impl TryToSql for UpsertBuilder<'_> {
    fn try_to_sql(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_upsert_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── CreateFromMeta ──────────────────────────────────────────────────────

impl TryToSql for CreateFromMeta<'_> {
    fn try_to_sql(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
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
        Ok(sql)
    }
}

// ── DefineModelBuilder ──────────────────────────────────────────────────

impl TryToSql for DefineModelBuilder {
    fn try_to_sql(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_define_model_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── AlterModelBuilder ───────────────────────────────────────────────────

impl TryToSql for AlterModelBuilder<'_> {
    fn try_to_sql(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_alter_model_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── DropModelBuilder ────────────────────────────────────────────────────

impl TryToSql for DropModelBuilder<'_> {
    fn try_to_sql(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_drop_model_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── DefineIndexBuilder ──────────────────────────────────────────────────

impl TryToSql for DefineIndexBuilder {
    fn try_to_sql(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_define_index_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── DropIndexBuilder ────────────────────────────────────────────────────

impl TryToSql for DropIndexBuilder {
    fn try_to_sql(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_drop_index_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── GrantBuilder ────────────────────────────────────────────────────────

impl TryToSql for GrantBuilder {
    fn try_to_sql(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let _dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_grant_ir(&ir).map(|o| o.sql)
    }
}

// ── RevokeBuilder ───────────────────────────────────────────────────────

impl TryToSql for RevokeBuilder {
    fn try_to_sql(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let _dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_revoke_ir(&ir).map(|o| o.sql)
    }
}

// ===========================================================================
// TransactionBuilder SQL extension
// ===========================================================================

/// Extension trait adding `.to_sql()` to [`TransactionBuilder`].
///
/// Provides default implementations so that downstream implementations
/// are not broken by the addition of new methods.
pub trait TransactionSqlExt {
    /// Try to render a TransactionIR to SQL, returning an error on failure.
    fn try_to_sql(ir: &TransactionIR) -> Result<String, BackendError> {
        render::render_transaction_ir(ir).map(|o| o.sql)
    }
    /// Render a TransactionIR to SQL.
    fn to_sql(ir: &TransactionIR) -> String {
        match Self::try_to_sql(ir) {
            Ok(sql) => sql,
            Err(e) => format!(
                "/* SQL render error: {} */",
                sanitize_for_sql_comment(&e.to_string())
            ),
        }
    }
}

impl TransactionSqlExt for TransactionBuilder {}

// ===========================================================================
// CompoundSelectBuilder — compound queries (moved from dol-builder)
// ===========================================================================

/// A compound SELECT composed of multiple queries joined by set operations.
///
/// Stores `QueryIR` objects and renders the entire compound query in a single
/// pass with a shared `ParamCounter`, ensuring bind-parameter numbers are
/// globally unique (e.g. Postgres `$1, $2, …`) and all parts use the same
/// dialect.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until rendered via .to_sql() or .try_to_sql()"]
pub struct CompoundSelectBuilder {
    base: QueryIR,
    parts: Vec<(SetOpKind, QueryIR)>,
    order_by: Vec<OrderByExpr>,
    has_offset: bool,
    has_limit: bool,
}

impl CompoundSelectBuilder {
    /// Create from a `QueryIR`.
    pub fn new(base: QueryIR) -> Self {
        Self {
            base,
            parts: Vec::new(),
            order_by: Vec::new(),
            has_offset: false,
            has_limit: false,
        }
    }

    pub fn union(mut self, query: QueryIR) -> Self {
        self.parts.push((SetOpKind::Union, query));
        self
    }

    pub fn union_all(mut self, query: QueryIR) -> Self {
        self.parts.push((SetOpKind::UnionAll, query));
        self
    }

    pub fn intersect(mut self, query: QueryIR) -> Self {
        self.parts.push((SetOpKind::Intersect, query));
        self
    }

    pub fn intersect_all(mut self, query: QueryIR) -> Self {
        self.parts.push((SetOpKind::IntersectAll, query));
        self
    }

    pub fn except(mut self, query: QueryIR) -> Self {
        self.parts.push((SetOpKind::Except, query));
        self
    }

    pub fn except_all(mut self, query: QueryIR) -> Self {
        self.parts.push((SetOpKind::ExceptAll, query));
        self
    }

    pub fn op(mut self, kind: SetOpKind, query: QueryIR) -> Self {
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

impl TryToSql for CompoundSelectBuilder {
    fn try_to_sql(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());

        let compound_ir = CompoundQueryIR {
            base: Box::new(self.base.clone()),
            operations: self.parts.clone(),
            order_by: self.order_by.clone(),
            offset: if self.has_offset {
                Some(OffsetLimit::Param)
            } else {
                None
            },
            limit: if self.has_limit {
                Some(OffsetLimit::Param)
            } else {
                None
            },
        };

        render::render_compound_query_ir(&compound_ir, dialect).map(|o| o.sql)
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
        CompoundSelectBuilder::new(self.build()).union(other.build())
    }

    fn union_all(self, other: GetBuilder<'_>) -> CompoundSelectBuilder {
        CompoundSelectBuilder::new(self.build()).union_all(other.build())
    }

    fn intersect(self, other: GetBuilder<'_>) -> CompoundSelectBuilder {
        CompoundSelectBuilder::new(self.build()).intersect(other.build())
    }

    fn except(self, other: GetBuilder<'_>) -> CompoundSelectBuilder {
        CompoundSelectBuilder::new(self.build()).except(other.build())
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
    use dol_core::builder::ModelBuilderExt;
    use dol_core::ir::definition::FieldDef;
    use dol_core::model::{Field, FieldType, Model};

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
        use dol_core::builder::ModelDefineExt;
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
        use dol_core::builder::DefineIndexBuilder;
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
        use dol_core::builder::DropIndexBuilder;
        let sql = DropIndexBuilder::new("idx_users_email").to_sql(None);
        assert_eq!(sql, "DROP INDEX idx_users_email");
    }

    // -- GrantBuilder --

    #[test]
    fn grant_basic() {
        use dol_core::builder::control::{GrantBuilder, Privilege};
        let sql = GrantBuilder::new(Privilege::Select)
            .on("users")
            .to("app_reader")
            .to_sql(None);
        assert_eq!(sql, "GRANT SELECT ON users TO app_reader");
    }

    // -- RevokeBuilder --

    #[test]
    fn revoke_basic() {
        use dol_core::builder::control::{Privilege, RevokeBuilder};
        let sql = RevokeBuilder::new(Privilege::Insert)
            .on("users")
            .from("app_reader")
            .to_sql(None);
        assert_eq!(sql, "REVOKE INSERT ON users FROM app_reader");
    }

    // -- CompoundSelectBuilder param numbering --

    #[test]
    fn compound_select_params_globally_unique() {
        // Two sub-queries each with a WHERE param. The compound builder
        // renders them with a shared counter so params are globally unique.
        let base_ir = TEST_MODEL
            .get()
            .columns(&["id"])
            .where_eq("tenant_id")
            .build();
        let part_ir = TEST_MODEL.get().columns(&["id"]).where_eq("status").build();
        let sql = CompoundSelectBuilder::new(base_ir)
            .union(part_ir)
            .limit()
            .offset()
            .to_sql(Some(&pg()));
        // Base uses $1, part uses $2, OFFSET/LIMIT use $3/$4
        assert!(sql.contains("tenant_id = $1"), "base param: {sql}");
        assert!(sql.contains("status = $2"), "part param: {sql}");
        assert!(
            sql.contains("OFFSET $3"),
            "expected OFFSET $3 in output: {sql}"
        );
        assert!(
            sql.contains("LIMIT $4"),
            "expected LIMIT $4 in output: {sql}"
        );
    }

    #[test]
    fn compound_select_no_params_starts_at_one() {
        // Sub-queries with no WHERE params → LIMIT gets $1.
        let base_ir = TEST_MODEL.get().columns(&["id"]).build();
        let part_ir = TEST_MODEL.get().columns(&["id"]).build();
        let sql = CompoundSelectBuilder::new(base_ir)
            .union(part_ir)
            .limit()
            .to_sql(Some(&pg()));
        assert!(sql.contains("LIMIT $1"), "expected LIMIT $1, got: {sql}");
    }
}
