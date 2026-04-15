//! UPSERT (INSERT ... ON CONFLICT) query builder for `dol-query`.

use dol_expr::{Expr, field, param, raw_expr};
use dol_ir::{EntityRef, UpsertIR};

// ===========================================================================
// UpsertQuery
// ===========================================================================

/// A composable UPSERT builder that works with any entity source.
///
/// Construct via [`Query::from(...).upsert()`](crate::Query::upsert).
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct UpsertQuery {
    name: String,
    namespace: Option<String>,
    field_names: Option<Vec<String>>,
    fields: Vec<String>,
    conflict_fields: Vec<String>,
    conflict_constraint: Option<String>,
    update_fields: Vec<String>,
    do_nothing_flag: bool,
    conflict_filters: Vec<Expr>,
    returning: Vec<String>,
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
            do_nothing_flag: false,
            conflict_filters: Vec::new(),
            returning: Vec::new(),
        }
    }

    /// Include all entity fields in the INSERT column list.
    ///
    /// # Panics
    ///
    /// Panics if this query was constructed from a plain string without
    /// field metadata. Use `.columns()` instead for string-sourced queries.
    pub fn all_fields(mut self) -> Self {
        let names = self
            .field_names
            .as_ref()
            .expect("all_fields() requires an Entity source with field metadata");
        self.fields = names.clone();
        self
    }

    /// Specify which columns to insert.
    pub fn columns(mut self, cols: &[&str]) -> Self {
        self.fields = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set the conflict target columns: `ON CONFLICT (col1, col2)`.
    pub fn on_conflict(mut self, cols: &[&str]) -> Self {
        self.conflict_fields = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set the conflict target to a named constraint: `ON CONFLICT ON CONSTRAINT name`.
    pub fn on_conflict_constraint(mut self, name: &str) -> Self {
        self.conflict_constraint = Some(name.to_string());
        self
    }

    /// Set the columns to update on conflict: `DO UPDATE SET col = EXCLUDED.col`.
    pub fn do_update(mut self, cols: &[&str]) -> Self {
        self.update_fields = cols.iter().map(|s| s.to_string()).collect();
        self.do_nothing_flag = false;
        self
    }

    /// Use `DO NOTHING` on conflict.
    pub fn do_nothing(mut self) -> Self {
        self.do_nothing_flag = true;
        self.update_fields.clear();
        self
    }

    /// Specify columns to return.
    pub fn returning(mut self, cols: &[&str]) -> Self {
        self.returning = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add `RETURNING *`.
    pub fn returning_all(mut self) -> Self {
        self.returning = vec!["*".to_string()];
        self
    }

    /// Add a filter expression to the conflict's WHERE clause.
    pub fn conflict_filter(mut self, expr: Expr) -> Self {
        self.conflict_filters.push(expr);
        self
    }

    /// Add `column = $N` to the conflict WHERE clause.
    pub fn conflict_where_eq(mut self, column: &str) -> Self {
        self.conflict_filters.push(field(column).eq(param()));
        self
    }

    /// Add `column = <literal>` to the conflict WHERE clause.
    pub fn conflict_where_eq_literal(mut self, column: &str, literal: &str) -> Self {
        self.conflict_filters
            .push(field(column).eq(raw_expr(literal)));
        self
    }

    /// Total bind-parameter count for this UPSERT.
    pub fn param_count(&self) -> usize {
        self.fields.len()
    }

    /// Build the canonical [`UpsertIR`].
    pub fn build(self) -> UpsertIR {
        UpsertIR {
            target: EntityRef {
                name: self.name,
                namespace: self.namespace,
                alias: None,
            },
            fields: self.fields,
            conflict_fields: self.conflict_fields,
            conflict_constraint: self.conflict_constraint,
            update_fields: self.update_fields,
            do_nothing: self.do_nothing_flag,
            conflict_filters: self.conflict_filters,
            returning: self.returning,
        }
    }
}
