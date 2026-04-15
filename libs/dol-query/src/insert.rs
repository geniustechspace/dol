//! INSERT query builder for `dol-query`.

use dol_ir::{EntityRef, InsertIR};

// ===========================================================================
// InsertQuery
// ===========================================================================

/// A composable INSERT builder that works with any entity source.
///
/// Construct via [`Query::from(...).insert()`](crate::Query::insert).
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct InsertQuery {
    name: String,
    namespace: Option<String>,
    field_names: Option<Vec<String>>,
    fields: Vec<String>,
    row_count: usize,
    returning: Vec<String>,
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

    /// Include all entity fields in the INSERT column list.
    ///
    /// # Panics
    ///
    /// Panics if this query was constructed from a plain string without
    /// field metadata. Use `.columns()` instead for string-sourced queries.
    pub fn all_columns(mut self) -> Self {
        let names = self
            .field_names
            .as_ref()
            .expect("all_columns() requires an Entity source with field metadata");
        self.fields = names.clone();
        self
    }

    /// Specify which columns to insert.
    pub fn columns(mut self, cols: &[&str]) -> Self {
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
    pub fn param_count(&self) -> usize {
        self.fields.len() * self.row_count
    }

    /// Build the canonical [`InsertIR`].
    pub fn build(self) -> InsertIR {
        InsertIR {
            target: EntityRef {
                name: self.name,
                namespace: self.namespace,
                alias: None,
            },
            fields: self.fields,
            row_count: self.row_count,
            returning: self.returning,
        }
    }
}
