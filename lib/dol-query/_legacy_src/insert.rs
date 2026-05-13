//! INSERT query builder for `dol-query`.
//!
//! Pure data: lowering to a `dol_command::program::Program` is handled
//! by [`dol_command::lower_query::lower_insert`] (or via the
//! [`BuildProgram`](dol_command::lower_query::BuildProgram) trait).

use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};

// ===========================================================================
// InsertQuery
// ===========================================================================

/// A composable INSERT builder that works with any entity source.
///
/// Construct via [`Query::from(...).insert()`](crate::Query::insert).
///
/// All fields are `pub` so the lowering host crate (`dol-command` with the
/// `query` feature, default-on) can read them without going through
/// accessors.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until lowered to a Program"]
pub struct InsertQuery {
    /// Target entity name (last dotted segment).
    pub name: String,
    /// Optional namespace prefix.
    pub namespace: Option<String>,
    /// Optional list of all field names (used as a default when
    /// `fields` is empty).
    pub field_names: Option<Vec<String>>,
    /// Explicit columns to insert. When empty, falls back to
    /// `field_names`.
    pub fields: Vec<String>,
    /// Number of value-tuples to emit (each tuple binds one
    /// `Param` per field).
    pub row_count: usize,
    /// Column names to return (`*` for all).
    pub returning: Vec<String>,
}

impl InsertQuery {
    pub(crate) fn new(
        name: String,
        namespace: Option<String>,
        field_names: Option<Vec<String>>,
    ) -> Self {
        Self {
            name,
            namespace,
            field_names,
            fields: Vec::new(),
            row_count: 1,
            returning: Vec::new(),
        }
    }

    /// Specify which fields to insert.
    pub fn fields(mut self, cols: &[&str]) -> Self {
        self.fields = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set the number of rows to insert (generates N value-tuples).
    pub fn rows(mut self, count: usize) -> Self {
        self.row_count = count;
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

    /// Total bind-parameter count for this INSERT.
    // Both factors are bounded by user-input column-/row-counts; on a
    // 64-bit `usize`, exceeding `usize::MAX` would require a query with
    // >2^64 columns × rows, which is not representable in any caller.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn param_count(&self) -> usize {
        let field_count = if self.fields.is_empty() {
            self.field_names.as_ref().map_or(0, |n| n.len())
        } else {
            self.fields.len()
        };
        field_count * self.row_count
    }
}
