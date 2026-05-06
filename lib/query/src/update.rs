//! UPDATE query builder for `dol-query`.

use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};

use dol_expr::tree::{Expr, field_dyn};

// ===========================================================================
// UpdateQuery
// ===========================================================================

/// A composable UPDATE builder that works with any entity source.
///
/// Construct via [`Query::from(...).update()`](crate::Query::update).
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .try_build() is called"]
pub struct UpdateQuery {
    name: String,
    namespace: Option<String>,
    assignments: Vec<(String, Expr<'static>)>,
    filters: Vec<Expr<'static>>,
    returning: Vec<String>,
}

impl UpdateQuery {
    pub(crate) fn new(name: String, namespace: Option<String>) -> Self {
        Self {
            name,
            namespace,
            assignments: Vec::new(),
            filters: Vec::new(),
            returning: Vec::new(),
        }
    }

    /// Set a column to a bind parameter: `col = $N`.
    pub fn set(mut self, column: &str) -> Self {
        self.assignments.push((column.to_string(), Expr::Param));
        self
    }

    /// Set multiple fields to bind parameters: `field1 = $N, field2 = $N+1, ...`.
    pub fn set_fields(mut self, columns: &[&str]) -> Self {
        for &c in columns {
            self.assignments.push((c.to_string(), Expr::Param));
        }
        self
    }

    /// Increment a column: `col = col + $N`.
    // The `+` here is `Expr<'static>::Add` overloaded via `core::ops::Add`,
    // not integer arithmetic; clippy can't tell from the symbol.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn set_increment(mut self, column: &str) -> Self {
        let expr = field_dyn(column) + Expr::Param;
        self.assignments.push((column.to_string(), expr));
        self
    }

    /// Set a column to an arbitrary [`Expr`].
    pub fn set_expr(mut self, column: &str, expr: Expr<'static>) -> Self {
        self.assignments.push((column.to_string(), expr));
        self
    }

    /// Add an arbitrary filter expression to the WHERE clause.
    ///
    /// Multiple filters are AND-joined.
    pub fn filter(mut self, expr: Expr<'static>) -> Self {
        self.filters.push(expr);
        self
    }

    /// Add `RETURNING *`.
    pub fn returning_all(mut self) -> Self {
        self.returning = vec!["*".to_string()];
        self
    }

    /// Specify columns to return.
    pub fn returning(mut self, cols: &[&str]) -> Self {
        self.returning = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Build the arena-based IR as a [`dol_ir::Program`] containing a
    /// single [`dol_ir::Operation::Update`] referencing an arena
    /// [`ExprNode::Update`](dol_expr::expr::ExprNode::Update).
    ///
    /// Fallible: returns [`BuildError::SetValue`](crate::BuildError::SetValue) /
    /// [`BuildError::Filter`](crate::BuildError::Filter) when lowering an assignment RHS or the WHERE
    /// clause exhausts the default budget.
    pub fn try_build(self) -> Result<dol_ir::Program, crate::BuildError> {
        use dol_core::policy::{Budget, Limits};
        use dol_expr::expr::{ExprNode, UpdateNode};
        use dol_expr::lower::{lower_expr_with_budget, lower_filters};
        use dol_ir::TargetKind;
        use dol_ir::operation::Update;

        let mut arena = dol_expr::ExprArena::new();
        let mut interner = dol_expr::Interner::new();
        let mut budget = Budget::new(Limits::host());

        let target_str = interner.intern(&dol_expr::lower::qualified_name(
            &self.name,
            &self.namespace,
        ));

        let mut columns: smallvec::SmallVec<[dol_expr::ids::StrId; 8]> = smallvec::SmallVec::new();
        let mut values: smallvec::SmallVec<[dol_expr::ids::NodeId; 8]> = smallvec::SmallVec::new();
        for (col, expr) in &self.assignments {
            columns.push(interner.intern(col));
            let nid = lower_expr_with_budget(expr, &mut arena, &mut interner, &mut budget)
                .map_err(|cause| crate::BuildError::SetValue {
                    column: col.clone(),
                    cause,
                })?;
            values.push(nid);
        }

        let filter = lower_filters(&self.filters, &mut arena, &mut interner, &mut budget)
            .map_err(crate::BuildError::Filter)?;

        let returning: smallvec::SmallVec<[dol_expr::ids::NodeId; 4]> = self
            .returning
            .iter()
            .map(|r| {
                let col = interner.intern(r);
                let fid = arena.alloc_field(dol_expr::FieldNode {
                    namespace: None,
                    name: col,
                    steps: smallvec::SmallVec::new(),
                });
                arena.alloc(ExprNode::Field(fid))
            })
            .collect();

        let unode = UpdateNode {
            target: target_str,
            columns,
            values,
            filter,
            returning,
        };
        let uid = arena.alloc_update(unode);
        let body = arena.alloc(ExprNode::Update(uid));

        let target = crate::target::target_from_parts(
            &mut interner,
            TargetKind::Relation,
            &self.name,
            self.namespace.as_deref(),
        );
        let op: dol_ir::Operation = Update { target, node: body }.into();
        Ok(dol_ir::Program::new(op, arena, interner))
    }
}
