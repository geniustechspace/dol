//! Upsert query builder for `dol-query`.
//!
//! An upsert inserts a new record, or — when an existing record matches on
//! a designated set of fields — patches it in place (`then_patch`) or skips
//! the write entirely (`then_skip`). The "match" condition is store-neutral:
//! SQL backends may render it as `ON CONFLICT`, document stores as a unique
//! filter, KV stores as an `IF NOT EXISTS` precondition, etc.

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
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .try_build() is called"]
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

    /// Build the arena-based IR as a [`dol_ir::Program`] containing a
    /// single [`dol_ir::Operation::Upsert`] referencing an arena
    /// `ExprNode` of opcode [`dol_expr::expr::ExprOp::Upsert`].
    ///
    /// Infallible in current shape (the builder only emits `Param`
    /// placeholders and structural `EXCLUDED.col` field references), but
    /// returns `Result` for API consistency with the other builders.
    pub fn try_build(self) -> Result<dol_ir::Program, crate::BuildError> {
        use dol_expr::expr::{ConflictClause, UpsertNode};
        use dol_ir::TargetKind;
        use dol_ir::operation::Upsert;

        let mut arena = dol_expr::ExprArena::new();
        let mut interner = dol_expr::Interner::new();

        let fields = if self.fields.is_empty() {
            self.field_names.unwrap_or_default()
        } else {
            self.fields
        };

        let target_str = interner.intern(&dol_expr::lower::qualified_name(
            &self.name,
            &self.namespace,
        ));
        let columns: smallvec::SmallVec<[dol_expr::ids::StrId; 8]> =
            fields.iter().map(|f| interner.intern(f)).collect();

        // One Param per field.
        let values: smallvec::SmallVec<[dol_expr::ids::NodeId; 8]> =
            fields.iter().map(|_| arena.alloc_param()).collect();

        let returning: smallvec::SmallVec<[dol_expr::ids::NodeId; 4]> = self
            .returning
            .iter()
            .map(|r| {
                let col = interner.intern(r);
                let fid = arena.alloc_field(dol_expr::FieldNode {
                    namespace: None,
                    name: col,
                    steps: smallvec::SmallVec::new(),
                });
                arena.alloc_field_ref(fid)
            })
            .collect();

        let conflict = if self.then_skip_flag {
            Some(ConflictClause::DoNothing)
        } else if !self.update_fields.is_empty() {
            let assignments: smallvec::SmallVec<
                [(dol_expr::ids::StrId, dol_expr::ids::NodeId); 4],
            > = self
                .update_fields
                .iter()
                .map(|col| {
                    let col_id = interner.intern(col);
                    // EXCLUDED.col reference
                    let ns = interner.intern("EXCLUDED");
                    let c = interner.intern(col);
                    let fid = arena.alloc_field(dol_expr::FieldNode {
                        namespace: Some(ns),
                        name: c,
                        steps: smallvec::SmallVec::new(),
                    });
                    let val_id = arena.alloc_field_ref(fid);
                    (col_id, val_id)
                })
                .collect();
            Some(ConflictClause::DoUpdate { assignments })
        } else {
            None
        };

        let unode = UpsertNode {
            target: target_str,
            columns,
            values,
            returning,
            conflict,
        };
        let uid = arena.alloc_upsert(unode);
        let body = arena.alloc_upsert_ref(uid);

        let target = crate::target::target_from_parts(
            &mut interner,
            TargetKind::Relation,
            &self.name,
            self.namespace.as_deref(),
        );
        let op: dol_ir::Operation = Upsert { target, node: body }.into();
        Ok(dol_ir::Program::new(op, arena, interner))
    }
}
