//! UPSERT (INSERT ... ON CONFLICT) query builder for `dol-query`.

use dol_expr::tree::Expr;

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
    then_skip_flag: bool,
    conflict_filters: Vec<Expr<'static>>,
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

    /// Set the conflict target columns: `ON CONFLICT (col1, col2)`.
    pub fn match_on(mut self, cols: &[&str]) -> Self {
        self.conflict_fields = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set the conflict target to a named constraint: `ON CONFLICT ON CONSTRAINT name`.
    pub fn match_on_constraint(mut self, name: &str) -> Self {
        self.conflict_constraint = Some(name.to_string());
        self
    }

    /// Set the columns to update on conflict: `DO UPDATE SET col = EXCLUDED.col`.
    pub fn then_patch(mut self, cols: &[&str]) -> Self {
        self.update_fields = cols.iter().map(|s| s.to_string()).collect();
        self.then_skip_flag = false;
        self
    }

    /// Use `DO NOTHING` on conflict.
    pub fn then_skip(mut self) -> Self {
        self.then_skip_flag = true;
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

    /// Build the arena-based IR as a [`dol_ir::Statement`].
    ///
    /// Returns `(Statement, ExprArena, Interner)` — the arena and interner
    /// are needed by renderers to resolve expression references.
    pub fn build(self) -> (dol_ir::Statement, dol_expr::ExprArena, dol_expr::Interner) {
        use dol_expr::expr::{ConflictClause, ExprNode, UpsertNode};

        let mut arena = dol_expr::ExprArena::new();
        let mut interner = dol_expr::Interner::new();

        let fields = if self.fields.is_empty() {
            self.field_names.unwrap_or_default()
        } else {
            self.fields
        };

        let target = interner.intern(&dol_expr::lower::qualified_name(&self.name, &self.namespace));
        let columns: smallvec::SmallVec<[u32; 8]> =
            fields.iter().map(|f| interner.intern(f)).collect();

        // One Param per field.
        let values: smallvec::SmallVec<[u32; 8]> = fields
            .iter()
            .map(|_| arena.alloc(ExprNode::Param))
            .collect();

        let returning: smallvec::SmallVec<[u32; 4]> = self
            .returning
            .iter()
            .map(|r| {
                let col = interner.intern(r);
                let fid = arena.alloc_field(dol_expr::FieldNode {
                    namespace: None,
                    column: col,
                    steps: smallvec::SmallVec::new(),
                });
                arena.alloc(ExprNode::Field(fid))
            })
            .collect();

        let conflict = if self.then_skip_flag {
            Some(ConflictClause::DoNothing)
        } else if !self.update_fields.is_empty() {
            let assignments: smallvec::SmallVec<[(u32, u32); 4]> = self
                .update_fields
                .iter()
                .map(|col| {
                    let col_id = interner.intern(col);
                    // EXCLUDED.col reference
                    let ns = interner.intern("EXCLUDED");
                    let c = interner.intern(col);
                    let fid = arena.alloc_field(dol_expr::FieldNode {
                        namespace: Some(ns),
                        column: c,
                        steps: smallvec::SmallVec::new(),
                    });
                    let val_id = arena.alloc(ExprNode::Field(fid));
                    (col_id, val_id)
                })
                .collect();
            Some(ConflictClause::DoUpdate { assignments })
        } else {
            None
        };

        let node = UpsertNode {
            target,
            columns,
            values,
            returning,
            conflict,
        };

        (dol_ir::Statement::Upsert(node), arena, interner)
    }
}
