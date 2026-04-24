//! DELETE query builder for `dol-query`.

use dol_expr::tree::Expr;

// ===========================================================================
// DeleteQuery
// ===========================================================================

/// A composable DELETE builder that works with any entity source.
///
/// Construct via [`Query::from(...).delete()`](crate::Query::delete).
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
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

    /// Build the arena-based IR as a [`dol_ir::Statement`].
    ///
    /// Returns `(Statement, ExprArena, Interner)` — the arena and interner
    /// are needed by renderers to resolve expression references.
    pub fn build(self) -> (dol_ir::Statement, dol_expr::ExprArena, dol_expr::Interner) {
        use dol_expr::lower::lower_filters;
        use dol_expr::expr::{DeleteNode, ExprNode};

        let mut arena = dol_expr::ExprArena::new();
        let mut interner = dol_expr::Interner::new();

        let target = interner.intern(&dol_expr::lower::qualified_name(&self.name, &self.namespace));
        let filter = lower_filters(&self.filters, &mut arena, &mut interner);

        let returning: smallvec::SmallVec<[u32; 4]> = self
            .returning
            .iter()
            .map(|r| {
                let col = interner.intern(r);
                let fid = arena.alloc_field(dol_expr::FieldNode {
                    namespace: None,
                    column: col,
                    steps: smallvec::SmallVec::new(),
                });
                arena.alloc(ExprNode::Field(fid))
            })
            .collect();

        let node = DeleteNode {
            target,
            filter,
            returning,
        };

        (dol_ir::Statement::Delete(node), arena, interner)
    }
}

/// Deprecated alias for [`DeleteQuery`].
#[deprecated(note = "renamed to `DeleteQuery`")]
pub type RemoveQuery = DeleteQuery;
