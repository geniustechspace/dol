//! UPDATE query builder for `dol-query`.

use dol_expr::tree::{Expr, field_dyn};

// ===========================================================================
// UpdateQuery
// ===========================================================================

/// A composable UPDATE builder that works with any entity source.
///
/// Construct via [`Query::from(...).update()`](crate::Query::update).
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until .build() is called"]
pub struct UpdateQuery {
    name: String,
    namespace: Option<String>,
    assignments: Vec<(String, Expr<'static>)>,
    filters: Vec<Expr<'static>>,
    returning: Vec<String>,
}

impl UpdateQuery {
    pub(crate) fn new(name: String, namespace: Option<String>) -> Self {
        Self {
            name,
            namespace,
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

    /// Build the arena-based IR as a [`dol_ir::Program`] containing a
    /// single [`dol_ir::Operation::Update`] referencing an arena
    /// [`ExprNode::Update`](dol_expr::expr::ExprNode::Update).
    pub fn build(self) -> dol_ir::Program {
        use dol_expr::expr::{ExprNode, UpdateNode};
        use dol_expr::lower::{lower_expr, lower_filters};
        use dol_ir::TargetKind;
        use dol_ir::operation::Update;

        let mut arena = dol_expr::ExprArena::new();
        let mut interner = dol_expr::Interner::new();

        let target_str = interner.intern(&dol_expr::lower::qualified_name(
            &self.name,
            &self.namespace,
        ));

        let mut columns: smallvec::SmallVec<[dol_expr::ids::StrId; 8]> = smallvec::SmallVec::new();
        let mut values: smallvec::SmallVec<[dol_expr::ids::NodeId; 8]> = smallvec::SmallVec::new();
        for (col, expr) in &self.assignments {
            columns.push(interner.intern(col));
            values.push(
                lower_expr(expr, &mut arena, &mut interner)
                    .expect("dol-query UpdateQuery: lowering of SET expression failed"),
            );
        }

        let filter = lower_filters(&self.filters, &mut arena, &mut interner)
            .expect("dol-query UpdateQuery: lowering of WHERE failed");

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
                arena.alloc(ExprNode::Field(fid))
            })
            .collect();

        let unode = UpdateNode {
            target: target_str,
            columns,
            values,
            filter,
            returning,
        };
        let uid = arena.alloc_update(unode);
        let body = arena.alloc(ExprNode::Update(uid));

        let target = crate::target::target_from_parts(
            &mut interner,
            TargetKind::Relation,
            &self.name,
            self.namespace.as_deref(),
        );
        let op: dol_ir::Operation = Update { target, node: body }.into();
        dol_ir::Program::new(op, arena, interner)
    }
}
