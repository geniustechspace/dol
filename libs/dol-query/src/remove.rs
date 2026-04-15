//! REMOVE (DELETE) query builder for `dol-query`.

use dol_expr::{Expr, col, param, raw_expr};
use dol_ir::{EntityRef, RemoveIR};

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
    filters: Vec<Expr>,
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
    pub fn filter(mut self, expr: Expr) -> Self {
        self.filters.push(expr);
        self
    }

    /// Add `column = $N` to the WHERE clause (bind parameter).
    pub fn where_eq(mut self, column: &str) -> Self {
        self.filters.push(col(column).eq(param()));
        self
    }

    /// Add `column = <literal>` to the WHERE clause.
    pub fn where_eq_literal(mut self, column: &str, literal: &str) -> Self {
        self.filters.push(col(column).eq(raw_expr(literal)));
        self
    }

    /// Add a raw SQL filter to the WHERE clause.
    pub fn where_raw(mut self, sql: &str) -> Self {
        self.filters.push(raw_expr(sql));
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
    pub fn build(self) -> RemoveIR {
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
