//! REMOVE (DELETE) query builder for `dol-query`.

use dol_core::expr::Expr;
use dol_core::ir::{EntityRef, RemoveIR};

// ===========================================================================
// RemoveQuery
// ===========================================================================

/// A composable DELETE builder that works with any entity source.
///
/// Construct via [`Query::from(...).remove()`](crate::Query::remove).
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct RemoveQuery {
    name: String,
    namespace: Option<String>,
    filters: Vec<Expr<'static>>,
    returning: Vec<String>,
}

impl RemoveQuery {
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

    /// Build the canonical [`RemoveIR`].
    pub fn build(self) -> RemoveIR<'static> {
        RemoveIR {
            target: EntityRef {
                name: self.name,
                namespace: self.namespace,
                alias: None,
            },
            filters: self.filters,
            returning: self.returning,
        }
    }
}
