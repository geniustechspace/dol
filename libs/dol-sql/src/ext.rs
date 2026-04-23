//! Extension trait that adds `.render()` to DOL builders for SQL rendering.
//!
//! The [`Render`] trait bridges `dol-builder` (which produces IR) with `dol-sql`
//! (which renders IR to SQL strings), keeping the dependency arrow from
//! `dol-sql` → `dol-builder` rather than the reverse.

use crate::dialect::{self, Dialect};
use crate::render;
use dol_entity::{
    AlterEntityBuilder, CreateFromMeta, DefineEntityBuilder, DefineIndexBuilder, DefineTypeBuilder,
    DropEntityBuilder, DropIndexBuilder, DropTypeBuilder,
};
use dol_query::builder::control::{DefinePolicyBuilder, GrantBuilder, RevokeBuilder};
use dol_query::builder::mutation::{
    InsertBuilder, InsertSelectBuilder, RemoveBuilder, UpdateBuilder, UpsertBuilder,
};
use dol_query::builder::query::GetBuilder;
use dol_query::builder::transaction::TransactionBuilder;
use dol_core::expr::OrderByExpr;
use dol_core::expr::pass::semantic::ExprContext;
use dol_core::op::BackendError;
use dol_core::op::OffsetLimit;
use dol_core::op::query::{CompoundQuery, Query, SetOp};
use dol_core::op::transaction::Transaction;
use dol_core::session::{BuildError, BuildSession};

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

    /// Validate all expressions in `session` then render.
    ///
    /// Returns `Err(BuildError::Validation(_))` if any pass rejects an
    /// expression before SQL generation is attempted.
    /// Returns `Err(BuildError::Render(_))` if the backend renderer fails.
    ///
    /// The default implementation skips validation and delegates to
    /// [`render`](Render::render).  Builders that carry [`Expr`] fields
    /// override this to run per-clause validation first.
    fn render_with(
        &self,
        session: &BuildSession,
        dialect: Option<&Dialect>,
    ) -> Result<String, BuildError> {
        let _ = session;
        self.render(dialect).map_err(BuildError::Render)
    }
}

// ── GetBuilder ──────────────────────────────────────────────────────────

impl Render for GetBuilder<'_> {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.clone().build();
        render::render_query_ir(&ir, dialect).map(|o| o.sql)
    }

    fn render_with(
        &self,
        session: &BuildSession,
        dialect: Option<&Dialect>,
    ) -> Result<String, BuildError> {
        let ir = self.clone().build();
        session.check_exprs(&ir.projections, ExprContext::Select)?;
        session.check_exprs(&ir.filters,     ExprContext::Where)?;
        session.check_exprs(&ir.group_by,    ExprContext::GroupBy)?;
        session.check_exprs(&ir.having,      ExprContext::Having)?;
        let ob_exprs: Vec<_> = ir.order_by.iter().map(|o| o.expr.clone()).collect();
        session.check_exprs(&ob_exprs,       ExprContext::OrderBy)?;
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        render::render_query_ir(&ir, dialect).map(|o| o.sql).map_err(BuildError::Render)
    }
}

// ── InsertBuilder ───────────────────────────────────────────────────────

impl Render for InsertBuilder<'_> {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.clone().build();
        render::render_insert_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── InsertSelectBuilder ─────────────────────────────────────────────────

impl Render for InsertSelectBuilder<'_> {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.clone().build();
        render::render_insert_select_ir(&ir, dialect).map(|o| o.sql)
    }
}

// ── UpdateBuilder ───────────────────────────────────────────────────────

impl Render for UpdateBuilder<'_> {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.clone().build();
        render::render_update_ir(&ir, dialect).map(|o| o.sql)
    }

    fn render_with(
        &self,
        session: &BuildSession,
        dialect: Option<&Dialect>,
    ) -> Result<String, BuildError> {
        let ir = self.clone().build();
        let assign_exprs: Vec<_> = ir.assignments.iter().map(|(_, e)| e.clone()).collect();
        session.check_exprs(&assign_exprs, ExprContext::Select)?;
        session.check_exprs(&ir.filters,   ExprContext::Where)?;
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        render::render_update_ir(&ir, dialect).map(|o| o.sql).map_err(BuildError::Render)
    }
}

// ── RemoveBuilder ───────────────────────────────────────────────────────

impl Render for RemoveBuilder<'_> {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.clone().build();
        render::render_remove_ir(&ir, dialect).map(|o| o.sql)
    }

    fn render_with(
        &self,
        session: &BuildSession,
        dialect: Option<&Dialect>,
    ) -> Result<String, BuildError> {
        let ir = self.clone().build();
        session.check_exprs(&ir.filters, ExprContext::Where)?;
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        render::render_remove_ir(&ir, dialect).map(|o| o.sql).map_err(BuildError::Render)
    }
}

// ── UpsertBuilder ───────────────────────────────────────────────────────

impl Render for UpsertBuilder<'_> {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let ir = self.clone().build();
        render::render_upsert_ir(&ir, dialect).map(|o| o.sql)
    }

    fn render_with(
        &self,
        session: &BuildSession,
        dialect: Option<&Dialect>,
    ) -> Result<String, BuildError> {
        let ir = self.clone().build();
        session.check_exprs(&ir.conflict_filters, ExprContext::Where)?;
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        render::render_upsert_ir(&ir, dialect).map(|o| o.sql).map_err(BuildError::Render)
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
        for field in &model.fields {
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
        for constraint in &model.constraints {
            let owned = dol_core::op::OwnedEntityConstraint::from(constraint);
            parts.push(format!("  {}", render::render_model_constraint(&owned)));
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
/// Unlike other builders, `TransactionBuilder` produces `Transaction` directly
/// (not `self`), so rendering uses associated functions rather than `&self` methods.
pub trait TransactionRender {
    /// Render a Transaction to SQL for the given dialect.
    fn render(ir: &Transaction, dialect: Option<&Dialect>) -> Result<String, BackendError>;
}

impl TransactionRender for TransactionBuilder {
    fn render(ir: &Transaction, dialect: Option<&Dialect>) -> Result<String, BackendError> {
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
/// Stores `Query` objects and renders the entire compound query in a single
/// pass with a shared `ParamCounter`, ensuring bind-parameter numbers are
/// globally unique (e.g. Postgres `$1, $2, …`) and all parts use the same
/// dialect.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until rendered via .render()"]
pub struct CompoundSelectBuilder<'a> {
    base: Query<'a>,
    parts: Vec<(SetOp, Query<'a>)>,
    order_by: Vec<OrderByExpr<'a>>,
    has_offset: bool,
    has_limit: bool,
}

impl<'a> CompoundSelectBuilder<'a> {
    /// Create from a `Query`.
    pub fn new(base: Query<'a>) -> Self {
        Self {
            base,
            parts: Vec::new(),
            order_by: Vec::new(),
            has_offset: false,
            has_limit: false,
        }
    }

    pub fn union(mut self, query: Query<'a>) -> Self {
        self.parts.push((SetOp::Union, query));
        self
    }

    pub fn union_all(mut self, query: Query<'a>) -> Self {
        self.parts.push((SetOp::UnionAll, query));
        self
    }

    pub fn intersect(mut self, query: Query<'a>) -> Self {
        self.parts.push((SetOp::Intersect, query));
        self
    }

    pub fn intersect_all(mut self, query: Query<'a>) -> Self {
        self.parts.push((SetOp::IntersectAll, query));
        self
    }

    pub fn except(mut self, query: Query<'a>) -> Self {
        self.parts.push((SetOp::Except, query));
        self
    }

    pub fn except_all(mut self, query: Query<'a>) -> Self {
        self.parts.push((SetOp::ExceptAll, query));
        self
    }

    pub fn op(mut self, kind: SetOp, query: Query<'a>) -> Self {
        self.parts.push((kind, query));
        self
    }

    pub fn order_by(mut self, exprs: impl IntoIterator<Item = OrderByExpr<'a>>) -> Self {
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

impl Render for CompoundSelectBuilder<'_> {
    fn render(&self, dialect: Option<&Dialect>) -> Result<String, BackendError> {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());

        let compound_ir = CompoundQuery {
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
pub trait GetBuilderSetOps<'a> {
    /// Start a UNION compound query with this builder as the base.
    fn union(self, other: GetBuilder<'a>) -> CompoundSelectBuilder<'a>;
    /// Start a UNION ALL compound query.
    fn union_all(self, other: GetBuilder<'a>) -> CompoundSelectBuilder<'a>;
    /// Start an INTERSECT compound query.
    fn intersect(self, other: GetBuilder<'a>) -> CompoundSelectBuilder<'a>;
    /// Start an EXCEPT compound query.
    fn except(self, other: GetBuilder<'a>) -> CompoundSelectBuilder<'a>;
}

/// Deprecated alias — use [`GetBuilderSetOps`].
#[deprecated(note = "use `GetBuilderSetOps`")]
pub trait GetBuilderSqlExt<'a>: GetBuilderSetOps<'a> {}

#[allow(deprecated)]
impl<'a, T: GetBuilderSetOps<'a>> GetBuilderSqlExt<'a> for T {}

impl<'a> GetBuilderSetOps<'a> for GetBuilder<'a> {
    fn union(self, other: GetBuilder<'a>) -> CompoundSelectBuilder<'a> {
        CompoundSelectBuilder::new(self.build()).union(other.build())
    }

    fn union_all(self, other: GetBuilder<'a>) -> CompoundSelectBuilder<'a> {
        CompoundSelectBuilder::new(self.build()).union_all(other.build())
    }

    fn intersect(self, other: GetBuilder<'a>) -> CompoundSelectBuilder<'a> {
        CompoundSelectBuilder::new(self.build()).intersect(other.build())
    }

    fn except(self, other: GetBuilder<'a>) -> CompoundSelectBuilder<'a> {
        CompoundSelectBuilder::new(self.build()).except(other.build())
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[path = "ext_tests.rs"]
mod tests;
