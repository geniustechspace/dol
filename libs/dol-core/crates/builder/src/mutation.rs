//! Mutation builders — INSERT, INSERT SELECT, UPDATE, REMOVE, UPSERT.
//!
//! Each builder borrows a `&Model` and provides chainable configuration
//! and a `.build()` method to produce the canonical IR.
//!
//! For SQL rendering, import the `Render` extension trait from `dol-sql`.

use dol_entity::Entity;
use dol_expr::{Expr, field, param, raw_expr};
use dol_ir::{EntityRef, InsertIR, InsertSelectIR, RemoveIR, UpdateIR, UpsertIR};

use super::query::count_single_expr_params;

// ===========================================================================
// Helper
// ===========================================================================

/// Build a `EntityRef` from a `Model`'s static metadata.
fn entity_ref(model: &Entity) -> EntityRef {
    EntityRef {
        name: model.name.to_string(),
        namespace: model.namespace.map(|s| s.to_string()),
        alias: None,
    }
}

// ===========================================================================
// InsertBuilder
// ===========================================================================

/// Builder for `INSERT INTO ... VALUES ...` statements.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct InsertBuilder<'a> {
    model: &'a Entity,
    fields: Vec<String>,
    row_count: usize,
    returning: Vec<String>,
}

impl<'a> InsertBuilder<'a> {
    pub fn new(model: &'a Entity) -> Self {
        Self {
            model,
            fields: Vec::new(),
            row_count: 1,
            returning: Vec::new(),
        }
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
        let field_count = if self.fields.is_empty() {
            self.model.fields.len()
        } else {
            self.fields.len()
        };
        field_count * self.row_count
    }

    /// Build the canonical [`InsertIR`].
    ///
    /// When no columns have been set (via `.columns()`), all entity fields
    /// are included by default.
    pub fn build(&self) -> InsertIR {
        // Default: include all entity fields when none were specified.
        let fields = if self.fields.is_empty() {
            self.model.field_names().map(|s| s.to_string()).collect()
        } else {
            self.fields.clone()
        };

        InsertIR {
            target: entity_ref(self.model),
            fields,
            row_count: self.row_count,
            returning: self.returning.clone(),
        }
    }
}

// ===========================================================================
// InsertSelectBuilder
// ===========================================================================

/// Builder for `INSERT INTO ... SELECT ...` statements.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct InsertSelectBuilder<'a> {
    model: &'a Entity,
    fields: Vec<String>,
    source_query: String,
    returning: Vec<String>,
}

impl<'a> InsertSelectBuilder<'a> {
    pub fn new(model: &'a Entity) -> Self {
        Self {
            model,
            fields: Vec::new(),
            source_query: String::new(),
            returning: Vec::new(),
        }
    }

    /// Specify which columns to insert into.
    pub fn columns(mut self, cols: &[&str]) -> Self {
        self.fields = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set the source SELECT query string.
    pub fn from_select(mut self, query: &str) -> Self {
        self.source_query = query.to_string();
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

    /// Build the canonical [`InsertSelectIR`].
    pub fn build(&self) -> InsertSelectIR {
        InsertSelectIR {
            target: entity_ref(self.model),
            fields: self.fields.clone(),
            source_query: self.source_query.clone(),
            returning: self.returning.clone(),
        }
    }
}

// ===========================================================================
// UpdateBuilder
// ===========================================================================

/// Builder for `UPDATE ... SET ... WHERE ...` statements.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct UpdateBuilder<'a> {
    model: &'a Entity,
    assignments: Vec<(String, Expr)>,
    filters: Vec<Expr>,
    returning: Vec<String>,
}

impl<'a> UpdateBuilder<'a> {
    pub fn new(model: &'a Entity) -> Self {
        Self {
            model,
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
    ///
    /// Convenience for chaining multiple `.set()` calls.
    pub fn set_columns(mut self, columns: &[&str]) -> Self {
        for &c in columns {
            self.assignments.push((c.to_string(), Expr::Param));
        }
        self
    }

    /// Set a column to a literal SQL expression: `col = <literal>`.
    pub fn set_literal(mut self, column: &str, literal: &str) -> Self {
        self.assignments
            .push((column.to_string(), raw_expr(literal)));
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
    pub fn filter(mut self, expr: Expr) -> Self {
        self.filters.push(expr);
        self
    }

    /// Add `column = $N` to the WHERE clause (bind parameter).
    pub fn where_eq(mut self, column: &str) -> Self {
        self.filters.push(field(column).eq(param()));
        self
    }

    /// Add `column = <literal>` to the WHERE clause.
    pub fn where_eq_literal(mut self, column: &str, literal: &str) -> Self {
        self.filters.push(field(column).eq(raw_expr(literal)));
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

    /// Total bind-parameter count for this UPDATE.
    pub fn param_count(&self) -> usize {
        let set_params: usize = self
            .assignments
            .iter()
            .map(|(_, expr)| count_single_expr_params(expr))
            .sum();
        let filter_params: usize = self.filters.iter().map(count_single_expr_params).sum();
        set_params + filter_params
    }

    /// Build the canonical [`UpdateIR`].
    pub fn build(&self) -> UpdateIR {
        UpdateIR {
            target: entity_ref(self.model),
            assignments: self.assignments.clone(),
            filters: self.filters.clone(),
            returning: self.returning.clone(),
        }
    }
}

// ===========================================================================
// RemoveBuilder
// ===========================================================================

/// Builder for `DELETE FROM ... WHERE ...` statements.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct RemoveBuilder<'a> {
    model: &'a Entity,
    filters: Vec<Expr>,
    returning: Vec<String>,
}

impl<'a> RemoveBuilder<'a> {
    pub fn new(model: &'a Entity) -> Self {
        Self {
            model,
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
        self.filters.push(field(column).eq(param()));
        self
    }

    /// Add `column = <literal>` to the WHERE clause.
    pub fn where_eq_literal(mut self, column: &str, literal: &str) -> Self {
        self.filters.push(field(column).eq(raw_expr(literal)));
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

    /// Total bind-parameter count for this DELETE.
    pub fn param_count(&self) -> usize {
        self.filters.iter().map(count_single_expr_params).sum()
    }

    /// Build the canonical [`RemoveIR`].
    pub fn build(&self) -> RemoveIR {
        RemoveIR {
            target: entity_ref(self.model),
            filters: self.filters.clone(),
            returning: self.returning.clone(),
        }
    }
}

// ===========================================================================
// UpsertBuilder
// ===========================================================================

/// Builder for `INSERT ... ON CONFLICT ... DO UPDATE/NOTHING` statements.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct UpsertBuilder<'a> {
    model: &'a Entity,
    fields: Vec<String>,
    conflict_fields: Vec<String>,
    conflict_constraint: Option<String>,
    update_fields: Vec<String>,
    do_nothing_flag: bool,
    conflict_filters: Vec<Expr>,
    returning: Vec<String>,
}

impl<'a> UpsertBuilder<'a> {
    pub fn new(model: &'a Entity) -> Self {
        Self {
            model,
            fields: Vec::new(),
            conflict_fields: Vec::new(),
            conflict_constraint: None,
            update_fields: Vec::new(),
            do_nothing_flag: false,
            conflict_filters: Vec::new(),
            returning: Vec::new(),
        }
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

    /// Add a filter expression to the conflict's WHERE clause
    /// (the WHERE that qualifies the DO UPDATE SET).
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
    ///
    /// Counts the insert-row parameters plus any parameters inside
    /// conflict filter expressions.
    pub fn param_count(&self) -> usize {
        let insert_params = if self.fields.is_empty() {
            self.model.fields.len()
        } else {
            self.fields.len()
        };
        let conflict_params: usize = self
            .conflict_filters
            .iter()
            .map(count_single_expr_params)
            .sum();
        insert_params + conflict_params
    }

    /// Build the canonical [`UpsertIR`].
    ///
    /// When no columns have been set (via `.columns()`), all entity fields
    /// are included by default.
    pub fn build(&self) -> UpsertIR {
        // Default: include all entity fields when none were specified.
        let fields = if self.fields.is_empty() {
            self.model.field_names().map(|s| s.to_string()).collect()
        } else {
            self.fields.clone()
        };

        UpsertIR {
            target: entity_ref(self.model),
            fields,
            conflict_fields: self.conflict_fields.clone(),
            conflict_constraint: self.conflict_constraint.clone(),
            update_fields: self.update_fields.clone(),
            do_nothing: self.do_nothing_flag,
            conflict_filters: self.conflict_filters.clone(),
            returning: self.returning.clone(),
        }
    }
}
