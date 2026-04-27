//! INSERT query builder for `dol-query`.

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

    /// Build the arena-based IR as a [`dol_ir::Program`] containing a
    /// single [`dol_ir::Operation::Insert`] referencing an arena
    /// [`ExprNode::Insert`](dol_expr::expr::ExprNode::Insert).
    pub fn build(self) -> dol_ir::Program {
        use dol_expr::expr::{ExprNode, InsertNode};
        use dol_ir::TargetKind;
        use dol_ir::operation::{Insert, InsertSource};

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
        let columns: smallvec::SmallVec<[u32; 8]> =
            fields.iter().map(|f| interner.intern(f)).collect();

        // Generate one Param node per field per row.
        let mut values = smallvec::SmallVec::new();
        for _ in 0..(self.row_count * fields.len()) {
            values.push(arena.alloc(ExprNode::Param));
        }

        // Returning columns as field-reference expressions.
        let returning: smallvec::SmallVec<[u32; 4]> = self
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

        let inode = InsertNode {
            target: target_str,
            columns,
            values,
            returning,
            conflict: None,
        };
        let iid = arena.alloc_insert(inode);
        let body = arena.alloc(ExprNode::Insert(iid));

        let target = crate::target::target_from_parts(
            &mut interner,
            TargetKind::Relation,
            &self.name,
            self.namespace.as_deref(),
        );
        let op: dol_ir::Operation = Insert {
            target,
            source: InsertSource::Node(body),
            returning: None,
        }
        .into();
        dol_ir::Program::new(op, arena, interner)
    }
}
