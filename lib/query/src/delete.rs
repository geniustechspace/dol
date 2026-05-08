//! DELETE query builder for `dol-query`.
//!
//! Pure data: the builder accumulates name / namespace / filters /
//! returning columns. Lowering to a `dol_command::program::Program` is
//! handled by [`dol_command::lower_query::lower_delete`] (or via the
//! [`BuildProgram`](dol_command::lower_query::BuildProgram) trait, which
//! provides the historical `.try_build()` method).

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
///
/// All fields are `pub` so the lowering host crate (`dol-command` with the
/// `query` feature, default-on) can read them without going through
/// accessors. Treat them as inputs to the lowering, not as a stable
/// rebindable surface.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until lowered to a Program"]
pub struct DeleteQuery {
    /// Target entity name (last dotted segment).
    pub name: String,
    /// Optional namespace prefix (everything before the last dotted
    /// segment of the source string).
    pub namespace: Option<String>,
    /// Filter expressions; AND-joined when lowered.
    pub filters: Vec<Expr<'static>>,
    /// Column names to return (`*` for all).
    pub returning: Vec<String>,
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
}

// `RemoveQuery` was the deprecated alias for `DeleteQuery`. Removed.
