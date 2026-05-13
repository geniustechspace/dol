//! Upsert query builder for `dol-query`.
//!
//! An upsert inserts a new record, or — when an existing record matches on
//! a designated set of fields — patches it in place (`then_patch`) or skips
//! the write entirely (`then_skip`). The "match" condition is store-neutral:
//! SQL backends may render it as `ON CONFLICT`, document stores as a unique
//! filter, KV stores as an `IF NOT EXISTS` precondition, etc.
//!
//! Pure data: lowering to a `dol_command::program::Program` is handled
//! by [`dol_command::lower_query::lower_upsert`] (or via the
//! [`BuildProgram`](dol_command::lower_query::BuildProgram) trait).

use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};

use dol_expr::tree::Expr;

// ===========================================================================
// UpsertQuery
// ===========================================================================

/// A composable upsert builder that works with any entity source.
///
/// Construct via [`Query::from(...).upsert()`](crate::Query::upsert).
///
/// All fields are `pub` so the lowering host crate (`dol-command` with the
/// `query` feature, default-on) can read them without going through
/// accessors.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until lowered to a Program"]
pub struct UpsertQuery {
    /// Target entity name (last dotted segment).
    pub name: String,
    /// Optional namespace prefix.
    pub namespace: Option<String>,
    /// Optional list of all field names (default for `fields` when empty).
    pub field_names: Option<Vec<String>>,
    /// Explicit columns to insert.
    pub fields: Vec<String>,
    /// Match-on column list (e.g. SQL `ON CONFLICT (a, b)`).
    pub conflict_fields: Vec<String>,
    /// Optional named constraint to match on.
    pub conflict_constraint: Option<String>,
    /// Columns to update on conflict (with `EXCLUDED.col` references).
    pub update_fields: Vec<String>,
    /// When true, conflict is a no-op (`DO NOTHING`).
    pub then_skip_flag: bool,
    /// Optional WHERE clause restricting which conflicts get patched.
    pub conflict_filters: Vec<Expr<'static>>,
    /// Column names to return (`*` for all).
    pub returning: Vec<String>,
}

impl UpsertQuery {
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
            conflict_fields: Vec::new(),
            conflict_constraint: None,
            update_fields: Vec::new(),
            then_skip_flag: false,
            conflict_filters: Vec::new(),
            returning: Vec::new(),
        }
    }

    /// Specify which fields to insert.
    pub fn fields(mut self, cols: &[&str]) -> Self {
        self.fields = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Match an existing record by these fields.
    pub fn match_on(mut self, cols: &[&str]) -> Self {
        self.conflict_fields = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Match an existing record by a named constraint.
    pub fn match_on_constraint(mut self, name: &str) -> Self {
        self.conflict_constraint = Some(name.to_string());
        self
    }

    /// On a match, patch the existing record's listed fields with the new
    /// values.
    pub fn then_patch(mut self, cols: &[&str]) -> Self {
        self.update_fields = cols.iter().map(|s| s.to_string()).collect();
        self.then_skip_flag = false;
        self
    }

    /// On a match, skip the write entirely.
    pub fn then_skip(mut self) -> Self {
        self.then_skip_flag = true;
        self.update_fields.clear();
        self
    }

    /// Return the listed fields from the affected records.
    pub fn returning(mut self, cols: &[&str]) -> Self {
        self.returning = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Return all fields from the affected records.
    pub fn returning_all(mut self) -> Self {
        self.returning = vec!["*".to_string()];
        self
    }

    /// Add a filter expression to the conflict's WHERE clause.
    pub fn conflict_filter(mut self, expr: Expr<'static>) -> Self {
        self.conflict_filters.push(expr);
        self
    }

    /// Total bind-parameter count for this UPSERT.
    pub fn param_count(&self) -> usize {
        if self.fields.is_empty() {
            self.field_names.as_ref().map_or(0, |n| n.len())
        } else {
            self.fields.len()
        }
    }
}
