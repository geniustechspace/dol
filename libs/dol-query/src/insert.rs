//! INSERT query builder for `dol-query`.

use dol_core::ir::{EntityRef, InsertIR};

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
    pub fn param_count(&self) -> usize {
        let field_count = if self.fields.is_empty() {
            self.field_names.as_ref().map_or(0, |n| n.len())
        } else {
            self.fields.len()
        };
        field_count * self.row_count
    }

    /// Build the canonical [`InsertIR`].
    ///
    /// When no fields have been set (via `.fields()`) and Entity field
    /// metadata is available, all entity fields are included by default.
    pub fn build(self) -> InsertIR {
        // Default: include all entity fields when none were specified.
        let fields = if self.fields.is_empty() {
            self.field_names.unwrap_or_default()
        } else {
            self.fields
        };

        InsertIR {
            target: EntityRef {
                name: self.name,
                namespace: self.namespace,
                alias: None,
            },
            fields,
            row_count: self.row_count,
            returning: self.returning,
        }
    }
}
