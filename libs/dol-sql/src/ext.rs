//! Extension trait that adds `.render()` to DOL builders for SQL rendering.
//!
//! The [`Render`] trait bridges `dol-builder` (which produces IR) with `dol-sql`
//! (which renders IR to SQL strings), keeping the dependency arrow from
//! `dol-sql` → `dol-builder` rather than the reverse.

use crate::dialect::{self, Dialect};
use crate::render;
use dol_core::builder::control::{DefinePolicyBuilder, GrantBuilder, RevokeBuilder};
use dol_core::builder::definition::{
    AlterEntityBuilder, CreateFromMeta, DefineEntityBuilder, DefineIndexBuilder, DefineTypeBuilder,
    DropEntityBuilder, DropIndexBuilder, DropTypeBuilder,
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
// Render — fallible SQL rendering trait
// ===========================================================================

/// Extension trait that renders DOL builders to SQL strings.
///
/// Bridges `dol-builder` (which produces IR) with `dol-sql`
/// (which renders IR to SQL), keeping the dependency arrow from
/// `dol-sql` → `dol-builder`.
pub trait Render {
    /// Render to a SQL string for the given dialect.
    /// Pass `None` for the global default dialect.
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError>;
}

// ── GetBuilder ──────────────────────────────────────────────────────────

impl Render for GetBuilder<'_> {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.clone().build();
        render::render_query_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── InsertBuilder ───────────────────────────────────────────────────────

impl Render for InsertBuilder<'_> {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_insert_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── InsertSelectBuilder ─────────────────────────────────────────────────

impl Render for InsertSelectBuilder<'_> {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_insert_select_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── UpdateBuilder ───────────────────────────────────────────────────────

impl Render for UpdateBuilder<'_> {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_update_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── RemoveBuilder ───────────────────────────────────────────────────────

impl Render for RemoveBuilder<'_> {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_remove_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── UpsertBuilder ───────────────────────────────────────────────────────

impl Render for UpsertBuilder<'_> {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_upsert_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── CreateFromMeta ──────────────────────────────────────────────────────

impl Render for CreateFromMeta<'_> {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let model = self.get_entity();
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

// ── DefineEntityBuilder ──────────────────────────────────────────────────

impl Render for DefineEntityBuilder {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_define_entity_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── AlterEntityBuilder ───────────────────────────────────────────────────

impl Render for AlterEntityBuilder<'_> {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_alter_entity_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── DropEntityBuilder ────────────────────────────────────────────────────

impl Render for DropEntityBuilder<'_> {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_drop_entity_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── DefineIndexBuilder ──────────────────────────────────────────────────

impl Render for DefineIndexBuilder {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_define_index_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── DropIndexBuilder ────────────────────────────────────────────────────

impl Render for DropIndexBuilder {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_drop_index_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── GrantBuilder ────────────────────────────────────────────────────────

impl Render for GrantBuilder {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let _dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_grant_ir(&ir).map(|o| o.sql)
    }
}

// ── RevokeBuilder ───────────────────────────────────────────────────────

impl Render for RevokeBuilder {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let _dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_revoke_ir(&ir).map(|o| o.sql)
    }
}

// ── DefineTypeBuilder ──────────────────────────────────────────────────

impl Render for DefineTypeBuilder {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_define_type_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── DropTypeBuilder ────────────────────────────────────────────────────

impl Render for DropTypeBuilder {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_drop_type_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── DefinePolicyBuilder ────────────────────────────────────────────────

impl Render for DefinePolicyBuilder {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.build();
        render::render_define_policy_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ===========================================================================
// TransactionBuilder SQL rendering
// ===========================================================================

/// SQL rendering for transaction control statements.
///
/// Unlike other builders, `TransactionBuilder` produces `TransactionIR` directly
/// (not `self`), so rendering uses associated functions rather than `&self` methods.
pub trait TransactionRender {
    /// Render a TransactionIR to SQL for the given dialect.
    fn render(ir: &TransactionIR, dialect: Option<&Dialect>) -> Result<String, BackendError>;
}

impl TransactionRender for TransactionBuilder {
    fn render(ir: &TransactionIR, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let d = match dialect {
            Some(d) => d,
            None => dialect::default_dialect(),
        };
        render::render_transaction_ir(ir, d).map(|o| o.sql)
    }
}

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
#[must_use = "builders do nothing until rendered via .render()"]
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

impl Render for CompoundSelectBuilder {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
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
        Expr::Subquery(Render::render(self, None).unwrap_or_default())
    }

    fn as_scalar_with(&self, dialect: &Dialect) -> Expr {
        Expr::Subquery(Render::render(self, Some(dialect)).unwrap_or_default())
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use dol_core::builder::EntityBuilderExt;
    use dol_core::expr::{field, param};
    use dol_core::ir::definition::FieldDef;
    use dol_core::model::{Entity, Field, FieldType};

    fn pg() -> Dialect {
        Dialect::postgres()
    }

    static TEST_MODEL: Entity = Entity::new(
        "users",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("tenant_id", FieldType::Uuid),
            Field::new("email", FieldType::Text).unique(),
            Field::new("status", FieldType::Text).default("'active'"),
            Field::new("created_at", FieldType::Timestamp).default("NOW()"),
        ],
    );

    static NS_MODEL: Entity =
        Entity::new("users", &[Field::new("id", FieldType::Uuid).primary_key()])
            .with_namespace("auth");

    // -- GetBuilder --

    #[test]
    fn get_builder_render() {
        let sql = TEST_MODEL
            .get()
            .filter(field("status").eq(param()))
            .order_by_desc("email")
            .limit()
            .render(Some(&pg()))
            .unwrap();
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("FROM users"));
    }

    // -- InsertBuilder --

    #[test]
    fn insert_defaults() {
        let sql = TEST_MODEL.insert().render(Some(&pg())).unwrap();
        assert!(sql.contains("INSERT INTO users (id, tenant_id, email, status, created_at)"));
        assert!(sql.contains("VALUES ($1, $2, $3, $4, $5)"));
    }

    #[test]
    fn insert_specific_columns() {
        let sql = TEST_MODEL
            .insert()
            .fields(&["id", "email"])
            .render(Some(&pg()))
            .unwrap();
        assert!(sql.contains("(id, email)"));
        assert!(sql.contains("($1, $2)"));
    }

    #[test]
    fn insert_multiple_rows() {
        let sql = TEST_MODEL
            .insert()
            .fields(&["id", "email"])
            .rows(3)
            .render(Some(&pg()))
            .unwrap();
        assert!(sql.contains("($1, $2), ($3, $4), ($5, $6)"));
    }

    #[test]
    fn insert_returning_all() {
        let sql = TEST_MODEL
            .insert()
            .fields(&["id"])
            .returning_all()
            .render(None)
            .unwrap();
        assert!(sql.contains("RETURNING *"));
    }

    #[test]
    fn insert_with_namespace() {
        let sql = NS_MODEL.insert().fields(&["id"]).render(None).unwrap();
        assert!(sql.contains("INSERT INTO auth.users"));
    }

    // -- UpdateBuilder --

    #[test]
    fn update_set_and_where() {
        let sql = TEST_MODEL
            .update()
            .set("email")
            .set("status")
            .filter(field("id").eq(param()))
            .render(Some(&pg()))
            .unwrap();
        assert!(sql.contains("UPDATE users SET"));
        assert!(sql.contains("email = $1"));
        assert!(sql.contains("status = $2"));
        assert!(sql.contains("WHERE id = $3"));
    }

    // -- RemoveBuilder --

    #[test]
    fn remove_basic() {
        let sql = TEST_MODEL
            .remove()
            .filter(field("id").eq(param()))
            .render(Some(&pg()))
            .unwrap();
        assert!(sql.contains("DELETE FROM users"));
        assert!(sql.contains("WHERE id = $1"));
    }

    // -- UpsertBuilder --

    #[test]
    fn upsert_basic() {
        let sql = TEST_MODEL
            .upsert()
            .fields(&["id", "email", "status"])
            .on_conflict(&["id"])
            .do_update(&["email", "status"])
            .render(Some(&pg()))
            .unwrap();
        assert!(sql.contains("INSERT INTO users (id, email, status)"));
        assert!(sql.contains("ON CONFLICT (id)"));
        assert!(sql.contains("DO UPDATE SET"));
    }

    // -- CreateFromMeta --

    #[test]
    fn create_from_meta_basic() {
        let sql = TEST_MODEL.create().render(Some(&pg())).unwrap();
        assert!(sql.starts_with("CREATE TABLE users ("));
        assert!(sql.contains("id UUID NOT NULL"));
        assert!(sql.contains("email TEXT NOT NULL UNIQUE"));
        assert!(sql.contains("status TEXT NOT NULL DEFAULT 'active'"));
        assert!(sql.contains("PRIMARY KEY (id)"));
    }

    #[test]
    fn create_from_meta_if_not_exists() {
        let sql = TEST_MODEL.create().if_not_exists().render(None).unwrap();
        assert!(sql.starts_with("CREATE TABLE IF NOT EXISTS users ("));
    }

    // -- DefineEntityBuilder --

    #[test]
    fn define_model_builder_basic() {
        use dol_core::builder::EntityDefineExt;
        let sql = Entity::define("sessions")
            .field(FieldDef::new("id", FieldType::Uuid).primary_key())
            .field(FieldDef::new("user_id", FieldType::Uuid))
            .if_not_exists()
            .render(Some(&pg()))
            .unwrap();
        assert!(sql.starts_with("CREATE TABLE IF NOT EXISTS sessions ("));
        assert!(sql.contains("id UUID NOT NULL"));
    }

    // -- AlterEntityBuilder --

    #[test]
    fn alter_model_add_and_drop() {
        let sql = TEST_MODEL
            .alter()
            .add_field(FieldDef::new("phone", FieldType::Text).nullable())
            .drop_field("legacy")
            .render(Some(&pg()))
            .unwrap();
        assert!(sql.contains("ADD COLUMN phone TEXT"));
        assert!(sql.contains("DROP COLUMN legacy"));
    }

    // -- DropEntityBuilder --

    #[test]
    fn drop_model_basic() {
        let sql = TEST_MODEL.drop_entity().render(None).unwrap();
        assert_eq!(sql, "DROP TABLE users");
    }

    #[test]
    fn drop_model_if_exists_cascade() {
        let sql = TEST_MODEL
            .drop_entity()
            .if_exists()
            .cascade()
            .render(None)
            .unwrap();
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
            .render(None)
            .unwrap();
        assert!(sql.contains("CREATE UNIQUE INDEX"));
        assert!(sql.contains("ON users"));
    }

    // -- DropIndexBuilder --

    #[test]
    fn drop_index_basic() {
        use dol_core::builder::DropIndexBuilder;
        let sql = DropIndexBuilder::new("idx_users_email")
            .render(None)
            .unwrap();
        assert_eq!(sql, "DROP INDEX idx_users_email");
    }

    // -- GrantBuilder --

    #[test]
    fn grant_basic() {
        use dol_core::builder::control::{GrantBuilder, Privilege};
        let sql = GrantBuilder::new(Privilege::Select)
            .on("users")
            .to("app_reader")
            .render(None)
            .unwrap();
        assert_eq!(sql, "GRANT SELECT ON users TO app_reader");
    }

    // -- RevokeBuilder --

    #[test]
    fn revoke_basic() {
        use dol_core::builder::control::{Privilege, RevokeBuilder};
        let sql = RevokeBuilder::new(Privilege::Insert)
            .on("users")
            .from("app_reader")
            .render(None)
            .unwrap();
        assert_eq!(sql, "REVOKE INSERT ON users FROM app_reader");
    }

    // -- CompoundSelectBuilder param numbering --

    #[test]
    fn compound_select_params_globally_unique() {
        // Two sub-queries each with a WHERE param. The compound builder
        // renders them with a shared counter so params are globally unique.
        let base_ir = TEST_MODEL
            .get()
            .fields(&["id"])
            .filter(field("tenant_id").eq(param()))
            .build();
        let part_ir = TEST_MODEL.get().fields(&["id"]).filter(field("status").eq(param())).build();
        let sql = CompoundSelectBuilder::new(base_ir)
            .union(part_ir)
            .limit()
            .offset()
            .render(Some(&pg()))
            .unwrap();
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
        let base_ir = TEST_MODEL.get().fields(&["id"]).build();
        let part_ir = TEST_MODEL.get().fields(&["id"]).build();
        let sql = CompoundSelectBuilder::new(base_ir)
            .union(part_ir)
            .limit()
            .render(Some(&pg()))
            .unwrap();
        assert!(sql.contains("LIMIT $1"), "expected LIMIT $1, got: {sql}");
    }

    // -- DefineTypeBuilder --

    #[test]
    fn define_type_postgres() {
        use dol_core::builder::DefineTypeBuilder;
        let sql = DefineTypeBuilder::new("order_status")
            .variant("pending")
            .variant("shipped")
            .variant("delivered")
            .render(Some(&pg()))
            .unwrap();
        assert_eq!(
            sql,
            "CREATE TYPE order_status AS ENUM ('pending', 'shipped', 'delivered')"
        );
    }

    #[test]
    fn define_type_mysql_is_comment() {
        use dol_core::builder::DefineTypeBuilder;
        let sql = DefineTypeBuilder::new("order_status")
            .variant("pending")
            .variant("shipped")
            .render(Some(&Dialect::mysql()))
            .unwrap();
        assert!(
            sql.starts_with("--"),
            "MySQL type should be a comment: {sql}"
        );
        assert!(sql.contains("ENUM"), "Should mention ENUM: {sql}");
    }

    #[test]
    fn define_type_sqlite_is_comment() {
        use dol_core::builder::DefineTypeBuilder;
        let sql = DefineTypeBuilder::new("order_status")
            .variant("pending")
            .render(Some(&Dialect::sqlite()))
            .unwrap();
        assert!(
            sql.starts_with("--"),
            "SQLite type should be a comment: {sql}"
        );
        assert!(sql.contains("CHECK"), "Should mention CHECK: {sql}");
    }

    #[test]
    fn define_type_with_namespace() {
        use dol_core::builder::DefineTypeBuilder;
        let sql = DefineTypeBuilder::new("order_status")
            .namespace("public")
            .variant("pending")
            .variant("shipped")
            .render(Some(&pg()))
            .unwrap();
        assert!(
            sql.contains("public.order_status"),
            "Expected qualified name: {sql}"
        );
    }

    #[test]
    fn define_type_variants_batch() {
        use dol_core::builder::DefineTypeBuilder;
        let sql = DefineTypeBuilder::new("color")
            .variants(&["red", "green", "blue"])
            .render(Some(&pg()))
            .unwrap();
        assert_eq!(sql, "CREATE TYPE color AS ENUM ('red', 'green', 'blue')");
    }

    // -- DropTypeBuilder --

    #[test]
    fn drop_type_postgres() {
        use dol_core::builder::DropTypeBuilder;
        let sql = DropTypeBuilder::new("order_status")
            .if_exists()
            .render(Some(&pg()))
            .unwrap();
        assert_eq!(sql, "DROP TYPE IF EXISTS order_status");
    }

    #[test]
    fn drop_type_without_if_exists() {
        use dol_core::builder::DropTypeBuilder;
        let sql = DropTypeBuilder::new("order_status")
            .render(Some(&pg()))
            .unwrap();
        assert_eq!(sql, "DROP TYPE order_status");
    }

    #[test]
    fn drop_type_mysql_is_comment() {
        use dol_core::builder::DropTypeBuilder;
        let sql = DropTypeBuilder::new("order_status")
            .render(Some(&Dialect::mysql()))
            .unwrap();
        assert!(
            sql.starts_with("--"),
            "MySQL drop type should be a comment: {sql}"
        );
    }

    // -- DefinePolicyBuilder --

    #[test]
    fn define_policy_postgres() {
        use dol_core::builder::DefinePolicyBuilder;
        use dol_core::expr::{field, param};
        use dol_core::ir::control::PolicyAction;

        let sql = DefinePolicyBuilder::new("tenant_isolation")
            .on("orders")
            .for_action(PolicyAction::All)
            .using(field("tenant_id").eq(param()))
            .check(field("tenant_id").eq(param()))
            .render(Some(&pg()))
            .unwrap();
        assert!(
            sql.contains("CREATE POLICY tenant_isolation ON orders FOR ALL"),
            "{sql}"
        );
        assert!(sql.contains("USING (tenant_id = $1)"), "{sql}");
        assert!(sql.contains("WITH CHECK (tenant_id = $2)"), "{sql}");
    }

    #[test]
    fn define_policy_read_only() {
        use dol_core::builder::DefinePolicyBuilder;
        use dol_core::expr::{field, lit};
        use dol_core::ir::control::PolicyAction;

        let sql = DefinePolicyBuilder::new("public_read")
            .on("posts")
            .for_action(PolicyAction::Read)
            .using(field("published").eq(lit(true)))
            .render(Some(&pg()))
            .unwrap();
        assert!(sql.contains("FOR SELECT"), "{sql}");
        assert!(sql.contains("USING (published = TRUE)"), "{sql}");
        assert!(
            !sql.contains("WITH CHECK"),
            "Read-only policy should not have WITH CHECK: {sql}"
        );
    }

    #[test]
    fn define_policy_no_expressions() {
        use dol_core::builder::DefinePolicyBuilder;
        use dol_core::ir::control::PolicyAction;

        let sql = DefinePolicyBuilder::new("allow_all")
            .on("logs")
            .for_action(PolicyAction::All)
            .render(Some(&pg()))
            .unwrap();
        assert_eq!(sql, "CREATE POLICY allow_all ON logs FOR ALL");
    }

    // -- Transaction block --

    #[test]
    fn transaction_block_postgres() {
        use dol_core::builder::transaction::TransactionBuilder;

        let insert_ir = TEST_MODEL.insert().fields(&["id", "email"]).build();
        let stmts = vec![dol_core::ir::Statement::Insert(insert_ir)];
        let ir = TransactionBuilder::block(stmts);
        let sql = TransactionBuilder::render(&ir, Some(&pg())).unwrap();
        assert!(sql.starts_with("BEGIN"), "Should start with BEGIN: {sql}");
        assert!(sql.contains("INSERT INTO"), "Should contain INSERT: {sql}");
        assert!(sql.ends_with("COMMIT"), "Should end with COMMIT: {sql}");
    }
}
