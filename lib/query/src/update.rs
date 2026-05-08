//! UPDATE query builder for `dol-query`.
//!
//! Pure data: lowering to a `dol_command::program::Program` is handled
//! by [`dol_command::lower_query::lower_update`] (or via the
//! [`BuildProgram`](dol_command::lower_query::BuildProgram) trait).

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
///
/// All fields are `pub` so the lowering host crate (`dol-command` with the
/// `query` feature, default-on) can read them without going through
/// accessors.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until lowered to a Program"]
pub struct UpdateQuery {
    /// Target entity name (last dotted segment).
    pub name: String,
    /// Optional namespace prefix.
    pub namespace: Option<String>,
    /// Column-name → assignment-expression pairs.
    pub assignments: Vec<(String, Expr<'static>)>,
    /// Filter expressions; AND-joined when lowered.
    pub filters: Vec<Expr<'static>>,
    /// Column names to return (`*` for all).
    pub returning: Vec<String>,
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
}
