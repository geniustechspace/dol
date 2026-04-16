//! UPDATE query builder for `dol-query`.

use dol_expr::{Expr, field};
use dol_ir::{EntityRef, UpdateIR};

// ===========================================================================
// UpdateQuery
// ===========================================================================

/// A composable UPDATE builder that works with any entity source.
///
/// Construct via [`Query::from(...).update()`](crate::Query::update).
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct UpdateQuery {
    name: String,
    namespace: Option<String>,
    assignments: Vec<(String, Expr)>,
    filters: Vec<Expr>,
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

    /// Set multiple columns to bind parameters: `col1 = $N, col2 = $N+1, ...`.
    pub fn set_columns(mut self, columns: &[&str]) -> Self {
        for &c in columns {
            self.assignments.push((c.to_string(), Expr::Param));
        }
        self
    }

    /// Set a column to a literal SQL expression: `col = <literal>`.
    pub fn set_literal(mut self, column: &str, literal: &str) -> Self {
        self.assignments
            .push((column.to_string(), dol_expr::raw_expr(literal)));
        self
    }

    /// Increment a column: `col = col + $N`.
    pub fn set_increment(mut self, column: &str) -> Self {
        let expr = field(column) + Expr::Param;
        self.assignments.push((column.to_string(), expr));
        self
    }

    /// Set a column to an arbitrary [`Expr`].
    pub fn set_expr(mut self, column: &str, expr: Expr) -> Self {
        self.assignments.push((column.to_string(), expr));
        self
    }

    /// Add an arbitrary filter expression to the WHERE clause.
    ///
    /// Multiple filters are AND-joined.
    pub fn filter(mut self, expr: Expr) -> Self {
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

    /// Build the canonical [`UpdateIR`].
    pub fn build(self) -> UpdateIR {
        UpdateIR {
            target: EntityRef {
                name: self.name,
                namespace: self.namespace,
                alias: None,
            },
            assignments: self.assignments,
            filters: self.filters,
            returning: self.returning,
        }
    }
}
