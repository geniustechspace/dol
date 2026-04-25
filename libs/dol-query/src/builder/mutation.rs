//! Mutation builders — INSERT, INSERT SELECT, UPDATE, DELETE, UPSERT.
//!
//! Each builder borrows a `&Model` and provides chainable configuration
//! and a `.build()` method to produce the canonical IR.
//!
//! For SQL rendering, import the `Render` extension trait from `dol-sql`.

use dol_expr::tree::{Expr, field_dyn};
use dol_schema::Entity;

use super::query::count_single_expr_params;

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

    /// Return all columns from the affected rows.
    pub fn output_all(mut self) -> Self {
        self.returning = vec!["*".to_string()];
        self
    }

    #[deprecated(note = "use `output_all()`")]
    #[inline]
    pub fn returning_all(self) -> Self {
        self.output_all()
    }

    /// Return specific columns from the affected rows.
    pub fn output(mut self, cols: &[&str]) -> Self {
        self.returning = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    #[deprecated(note = "use `output()`")]
    #[inline]
    pub fn returning(self, cols: &[&str]) -> Self {
        self.output(cols)
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

    /// Build the arena-based IR as a [`dol_ir::Statement`].
    ///
    /// When no fields have been set (via `.fields()`), all entity fields
    /// are included by default.
    pub fn build(self) -> (dol_ir::Statement, dol_expr::ExprArena, dol_expr::Interner) {
        let mut q = crate::InsertQuery::new(
            self.model.name.to_string(),
            self.model.namespace.as_ref().map(|s| s.to_string()),
            Some(self.model.field_names().map(|s| s.to_string()).collect()),
        );
        if !self.fields.is_empty() {
            q = q.fields(&self.fields.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        }
        q = q.rows(self.row_count);
        if !self.returning.is_empty() {
            q = q.returning(
                &self
                    .returning
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>(),
            );
        }
        q.build()
    }
}

// ===========================================================================
// InsertSelectBuilder
// ===========================================================================

/// INSERT ... SELECT ... operation.
#[derive(Debug, Clone)]
pub struct InsertSelect {
    pub target: dol_ir::EntityRef,
    pub fields: Vec<String>,
    pub source_query: String,
    pub returning: Vec<String>,
}

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

    /// Specify which fields to insert into.
    pub fn fields(mut self, cols: &[&str]) -> Self {
        self.fields = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set the source SELECT query string.
    pub fn from_select(mut self, query: &str) -> Self {
        self.source_query = query.to_string();
        self
    }

    /// Return specific columns from the affected rows.
    pub fn output(mut self, cols: &[&str]) -> Self {
        self.returning = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    #[deprecated(note = "use `output()`")]
    #[inline]
    pub fn returning(self, cols: &[&str]) -> Self {
        self.output(cols)
    }

    /// Return all columns from the affected rows.
    pub fn output_all(mut self) -> Self {
        self.returning = vec!["*".to_string()];
        self
    }

    #[deprecated(note = "use `output_all()`")]
    #[inline]
    pub fn returning_all(self) -> Self {
        self.output_all()
    }

    /// Build the [`InsertSelect`].
    pub fn build(self) -> InsertSelect {
        InsertSelect {
            target: dol_ir::EntityRef {
                name: self.model.name.to_string(),
                namespace: self.model.namespace.as_ref().map(|s| s.to_string()),
                alias: None,
            },
            fields: self.fields,
            source_query: self.source_query,
            returning: self.returning,
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
    assignments: Vec<(String, Expr<'static>)>,
    filters: Vec<Expr<'static>>,
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

    /// Set multiple fields to bind parameters: `field1 = $N, field2 = $N+1, ...`.
    ///
    /// Convenience for chaining multiple `.set()` calls.
    pub fn set_fields(mut self, columns: &[&str]) -> Self {
        for &c in columns {
            self.assignments.push((c.to_string(), Expr::Param));
        }
        self
    }

    /// Increment a column: `col = col + $N`.
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

    /// Return all columns from the affected rows.
    pub fn output_all(mut self) -> Self {
        self.returning = vec!["*".to_string()];
        self
    }

    #[deprecated(note = "use `output_all()`")]
    #[inline]
    pub fn returning_all(self) -> Self {
        self.output_all()
    }

    /// Return specific columns from the affected rows.
    pub fn output(mut self, cols: &[&str]) -> Self {
        self.returning = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    #[deprecated(note = "use `output()`")]
    #[inline]
    pub fn returning(self, cols: &[&str]) -> Self {
        self.output(cols)
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

    /// Build the arena-based IR as a [`dol_ir::Statement`].
    pub fn build(self) -> (dol_ir::Statement, dol_expr::ExprArena, dol_expr::Interner) {
        let mut q = crate::UpdateQuery::new(
            self.model.name.to_string(),
            self.model.namespace.as_ref().map(|s| s.to_string()),
        );
        for (col, expr) in self.assignments {
            q = q.set_expr(&col, expr);
        }
        for f in self.filters {
            q = q.filter(f);
        }
        if !self.returning.is_empty() {
            q = q.returning(
                &self
                    .returning
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>(),
            );
        }
        q.build()
    }
}

// ===========================================================================
// DeleteBuilder
// ===========================================================================

/// Builder for `DELETE FROM ... WHERE ...` statements.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct DeleteBuilder<'a> {
    model: &'a Entity,
    filters: Vec<Expr<'static>>,
    returning: Vec<String>,
}

impl<'a> DeleteBuilder<'a> {
    pub fn new(model: &'a Entity) -> Self {
        Self {
            model,
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

    /// Return all columns from the affected rows.
    pub fn output_all(mut self) -> Self {
        self.returning = vec!["*".to_string()];
        self
    }

    #[deprecated(note = "use `output_all()`")]
    #[inline]
    pub fn returning_all(self) -> Self {
        self.output_all()
    }

    /// Return specific columns from the affected rows.
    pub fn output(mut self, cols: &[&str]) -> Self {
        self.returning = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    #[deprecated(note = "use `output()`")]
    #[inline]
    pub fn returning(self, cols: &[&str]) -> Self {
        self.output(cols)
    }

    /// Total bind-parameter count for this DELETE.
    pub fn param_count(&self) -> usize {
        self.filters.iter().map(count_single_expr_params).sum()
    }

    /// Build the arena-based IR as a [`dol_ir::Statement`].
    pub fn build(self) -> (dol_ir::Statement, dol_expr::ExprArena, dol_expr::Interner) {
        let mut q = crate::DeleteQuery::new(
            self.model.name.to_string(),
            self.model.namespace.as_ref().map(|s| s.to_string()),
        );
        for f in self.filters {
            q = q.filter(f);
        }
        if !self.returning.is_empty() {
            q = q.returning(
                &self
                    .returning
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>(),
            );
        }
        q.build()
    }
}

/// Deprecated alias for [`DeleteBuilder`].
#[deprecated(note = "renamed to `DeleteBuilder`")]
pub type RemoveBuilder<'a> = DeleteBuilder<'a>;

// ===========================================================================
// UpsertBuilder
// ===========================================================================

/// Builder for upsert statements (`then_patch` or `then_skip` on a match).
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct UpsertBuilder<'a> {
    model: &'a Entity,
    fields: Vec<String>,
    conflict_fields: Vec<String>,
    conflict_constraint: Option<String>,
    update_fields: Vec<String>,
    then_skip_flag: bool,
    conflict_filters: Vec<Expr<'static>>,
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

    /// Set the match-target fields for the upsert.
    pub fn match_on(mut self, cols: &[&str]) -> Self {
        self.conflict_fields = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set the match target to a named constraint.
    pub fn match_on_constraint(mut self, name: &str) -> Self {
        self.conflict_constraint = Some(name.to_string());
        self
    }

    /// Specify the fields to patch when an existing record matches.
    pub fn then_patch(mut self, cols: &[&str]) -> Self {
        self.update_fields = cols.iter().map(|s| s.to_string()).collect();
        self.then_skip_flag = false;
        self
    }

    /// Skip silently when an existing record matches.
    pub fn then_skip(mut self) -> Self {
        self.then_skip_flag = true;
        self.update_fields.clear();
        self
    }

    /// Return specific fields from the affected records.
    pub fn output(mut self, cols: &[&str]) -> Self {
        self.returning = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Return all fields from the affected records.
    pub fn output_all(mut self) -> Self {
        self.returning = vec!["*".to_string()];
        self
    }

    #[deprecated(note = "use `output_all()`")]
    #[inline]
    pub fn returning_all(self) -> Self {
        self.output_all()
    }

    /// Add a filter expression that further qualifies which existing records
    /// the patch action applies to.
    pub fn match_filter(mut self, expr: Expr<'static>) -> Self {
        self.conflict_filters.push(expr);
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

    /// Build the arena-based IR as a [`dol_ir::Statement`].
    ///
    /// When no fields have been set (via `.fields()`), all entity fields
    /// are included by default.
    pub fn build(self) -> (dol_ir::Statement, dol_expr::ExprArena, dol_expr::Interner) {
        let field_names: Vec<String> = self.model.field_names().map(|s| s.to_string()).collect();
        let mut q = crate::UpsertQuery::new(
            self.model.name.to_string(),
            self.model.namespace.as_ref().map(|s| s.to_string()),
            Some(field_names),
        );
        if !self.fields.is_empty() {
            q = q.fields(&self.fields.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        }
        if !self.conflict_fields.is_empty() {
            q = q.match_on(
                &self
                    .conflict_fields
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>(),
            );
        }
        if let Some(ref constraint) = self.conflict_constraint {
            q = q.match_on_constraint(constraint);
        }
        if !self.update_fields.is_empty() {
            q = q.then_patch(
                &self
                    .update_fields
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>(),
            );
        }
        if self.then_skip_flag {
            q = q.then_skip();
        }
        for f in self.conflict_filters {
            q = q.conflict_filter(f);
        }
        if !self.returning.is_empty() {
            q = q.returning(
                &self
                    .returning
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>(),
            );
        }
        q.build()
    }
}
