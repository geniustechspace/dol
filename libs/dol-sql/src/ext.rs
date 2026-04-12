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
use dol_core::ir::query::SetOpKind;
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
pub trait TransactionSqlExt {
    /// Try to render a TransactionIR to SQL, returning an error on failure.
    fn try_to_sql(ir: &TransactionIR) -> Result<String, BackendError>;
    /// Render a TransactionIR to SQL.
    fn to_sql(ir: &TransactionIR) -> String;
}

impl TransactionSqlExt for TransactionBuilder {
    fn try_to_sql(ir: &TransactionIR) -> Result<String, BackendError> {
        render::render_transaction_ir(ir).map(|o| o.sql)
    }

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

// ===========================================================================
// CompoundSelectBuilder — compound queries (moved from dol-builder)
// ===========================================================================

/// A compound SELECT composed of multiple queries joined by set operations.
///
/// Each sub-query is stored as a pre-rendered SQL string alongside its
/// bind-parameter count so that ORDER BY / pagination parameters are numbered
/// correctly for dialects with numbered placeholders (e.g. Postgres `$N`).
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .to_sql() is called"]
pub struct CompoundSelectBuilder {
    base: String,
    base_param_count: usize,
    parts: Vec<(SetOpKind, String, usize)>,
    order_by: Vec<OrderByExpr>,
    has_offset: bool,
    has_limit: bool,
}

impl CompoundSelectBuilder {
    /// Create from a pre-rendered SQL string.
    ///
    /// If the base query contains bind parameters (e.g. `$1`, `$2`), use
    /// [`Self::new_with_param_count`] instead so that subsequent ORDER BY /
    /// pagination placeholders are numbered correctly.
    pub fn new(base_sql: String) -> Self {
        Self {
            base: base_sql,
            base_param_count: 0,
            parts: Vec::new(),
            order_by: Vec::new(),
            has_offset: false,
            has_limit: false,
        }
    }

    /// Create from a pre-rendered SQL string, recording its parameter count.
    pub fn new_with_param_count(base_sql: String, param_count: usize) -> Self {
        Self {
            base: base_sql,
            base_param_count: param_count,
            parts: Vec::new(),
            order_by: Vec::new(),
            has_offset: false,
            has_limit: false,
        }
    }

    pub fn union(mut self, query: String) -> Self {
        self.parts.push((SetOpKind::Union, query, 0));
        self
    }

    pub fn union_with_params(mut self, query: String, param_count: usize) -> Self {
        self.parts.push((SetOpKind::Union, query, param_count));
        self
    }

    pub fn union_all(mut self, query: String) -> Self {
        self.parts.push((SetOpKind::UnionAll, query, 0));
        self
    }

    pub fn union_all_with_params(mut self, query: String, param_count: usize) -> Self {
        self.parts.push((SetOpKind::UnionAll, query, param_count));
        self
    }

    pub fn intersect(mut self, query: String) -> Self {
        self.parts.push((SetOpKind::Intersect, query, 0));
        self
    }

    pub fn intersect_with_params(mut self, query: String, param_count: usize) -> Self {
        self.parts.push((SetOpKind::Intersect, query, param_count));
        self
    }

    pub fn intersect_all(mut self, query: String) -> Self {
        self.parts.push((SetOpKind::IntersectAll, query, 0));
        self
    }

    pub fn intersect_all_with_params(mut self, query: String, param_count: usize) -> Self {
        self.parts
            .push((SetOpKind::IntersectAll, query, param_count));
        self
    }

    pub fn except(mut self, query: String) -> Self {
        self.parts.push((SetOpKind::Except, query, 0));
        self
    }

    pub fn except_with_params(mut self, query: String, param_count: usize) -> Self {
        self.parts.push((SetOpKind::Except, query, param_count));
        self
    }

    pub fn except_all(mut self, query: String) -> Self {
        self.parts.push((SetOpKind::ExceptAll, query, 0));
        self
    }

    pub fn except_all_with_params(mut self, query: String, param_count: usize) -> Self {
        self.parts.push((SetOpKind::ExceptAll, query, param_count));
        self
    }

    pub fn op(mut self, kind: SetOpKind, query: String) -> Self {
        self.parts.push((kind, query, 0));
        self
    }

    pub fn op_with_params(mut self, kind: SetOpKind, query: String, param_count: usize) -> Self {
        self.parts.push((kind, query, param_count));
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
        let mut counter = dialect.param_counter();

        // Advance the counter past all parameters already present in the
        // pre-rendered sub-queries so ORDER BY / pagination placeholders
        // don't reuse numbers from the base or parts.
        let existing_params =
            self.base_param_count + self.parts.iter().map(|(_, _, c)| c).sum::<usize>();
        for _ in 0..existing_params {
            counter.next();
        }

        let mut sql = self.base.clone();

        for (kind, query, _) in &self.parts {
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

        Ok(sql)
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

/// Helper: render a `GetBuilder` and return both the SQL string and param count.
fn render_get_builder(builder: &GetBuilder<'_>) -> (String, usize) {
    let dialect = dialect::default_dialect();
    let ir = builder.clone().build();
    match render::render_query_ir(&ir, dialect) {
        Ok(output) => (output.sql, output.param_count),
        Err(e) => (
            format!(
                "/* SQL render error: {} */",
                sanitize_for_sql_comment(&e.to_string())
            ),
            0,
        ),
    }
}

impl GetBuilderSqlExt for GetBuilder<'_> {
    fn union(self, other: GetBuilder<'_>) -> CompoundSelectBuilder {
        let (base, base_params) = render_get_builder(&self);
        let (rhs, rhs_params) = render_get_builder(&other);
        CompoundSelectBuilder::new_with_param_count(base, base_params)
            .union_with_params(rhs, rhs_params)
    }

    fn union_all(self, other: GetBuilder<'_>) -> CompoundSelectBuilder {
        let (base, base_params) = render_get_builder(&self);
        let (rhs, rhs_params) = render_get_builder(&other);
        CompoundSelectBuilder::new_with_param_count(base, base_params)
            .union_all_with_params(rhs, rhs_params)
    }

    fn intersect(self, other: GetBuilder<'_>) -> CompoundSelectBuilder {
        let (base, base_params) = render_get_builder(&self);
        let (rhs, rhs_params) = render_get_builder(&other);
        CompoundSelectBuilder::new_with_param_count(base, base_params)
            .intersect_with_params(rhs, rhs_params)
    }

    fn except(self, other: GetBuilder<'_>) -> CompoundSelectBuilder {
        let (base, base_params) = render_get_builder(&self);
        let (rhs, rhs_params) = render_get_builder(&other);
        CompoundSelectBuilder::new_with_param_count(base, base_params)
            .except_with_params(rhs, rhs_params)
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
    fn compound_select_params_advance_past_subqueries() {
        // Two sub-queries each with 1 WHERE param (total 2 existing params).
        // OFFSET/LIMIT params should continue numbering after existing params.
        let base = "SELECT id FROM users WHERE tenant_id = $1";
        let part = "SELECT id FROM users WHERE status = $2";
        let sql = CompoundSelectBuilder::new_with_param_count(base.to_string(), 1)
            .union_with_params(part.to_string(), 1)
            .limit()
            .offset()
            .to_sql(Some(&pg()));
        // Original params $1 and $2 remain in the sub-queries
        assert!(sql.contains("tenant_id = $1"), "base param preserved");
        assert!(sql.contains("status = $2"), "part param preserved");
        // New params for OFFSET/LIMIT continue at $3, $4 (not $1, $2)
        assert!(sql.contains("$3"), "expected $3 in output, got: {sql}");
        assert!(sql.contains("$4"), "expected $4 in output, got: {sql}");
    }

    #[test]
    fn compound_select_zero_params_starts_at_one() {
        // When no param counts are provided (legacy API), counter starts at 1.
        let base = "SELECT id FROM users";
        let part = "SELECT id FROM archive";
        let sql = CompoundSelectBuilder::new(base.to_string())
            .union(part.to_string())
            .limit()
            .to_sql(Some(&pg()));
        assert!(sql.contains("LIMIT $1"), "expected LIMIT $1, got: {sql}");
    }
}
