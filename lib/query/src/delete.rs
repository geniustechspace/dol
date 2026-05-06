//! DELETE query builder for `dol-query`.

use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};

use dol_expr::tree::Expr;

// ===========================================================================
// DeleteQuery
// ===========================================================================

/// A composable DELETE builder that works with any entity source.
///
/// Construct via [`Query::from(...).delete()`](crate::Query::delete).
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .try_build() is called"]
pub struct DeleteQuery {
    name: String,
    namespace: Option<String>,
    filters: Vec<Expr<'static>>,
    returning: Vec<String>,
}

impl DeleteQuery {
    pub(crate) fn new(name: String, namespace: Option<String>) -> Self {
        Self {
            name,
            namespace,
            filters: Vec::new(),
            returning: Vec::new(),
        }
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
    /// single [`dol_ir::Operation::Delete`] referencing an arena
    /// [`ExprNode::Delete`](dol_expr::expr::ExprNode::Delete).
    ///
    /// Fallible: returns [`BuildError::Filter`] when lowering the WHERE
    /// clause exhausts the default [`Budget`](dol_core::policy::Budget)
    /// (depth or fuel cap from [`Limits::host`](dol_core::policy::Limits::host)).
    /// Callers needing a non-default budget can build the program manually
    /// using `dol_expr::lower::lower_filters` directly.
    pub fn try_build(self) -> Result<dol_ir::Program, crate::BuildError> {
        use dol_core::policy::{Budget, Limits};
        use dol_expr::expr::{DeleteNode, ExprNode};
        use dol_expr::lower::lower_filters;
        use dol_ir::TargetKind;
        use dol_ir::operation::Delete;

        let mut arena = dol_expr::ExprArena::new();
        let mut interner = dol_expr::Interner::new();
        let mut budget = Budget::new(Limits::host());

        let target_str = interner.intern(&dol_expr::lower::qualified_name(
            &self.name,
            &self.namespace,
        ));
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

        let dnode = DeleteNode {
            target: target_str,
            filter,
            returning,
        };
        let did = arena.alloc_delete(dnode);
        let body = arena.alloc(ExprNode::Delete(did));

        let target = crate::target::target_from_parts(
            &mut interner,
            TargetKind::Relation,
            &self.name,
            self.namespace.as_deref(),
        );
        let op: dol_ir::Operation = Delete { target, node: body }.into();
        Ok(dol_ir::Program::new(op, arena, interner))
    }
}

// `RemoveQuery` was the deprecated alias for `DeleteQuery`. Removed.
