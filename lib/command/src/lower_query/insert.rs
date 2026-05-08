//! Lower a [`dol_query::InsertQuery`] into an [`Operation::Insert`] program.

extern crate alloc;

use crate::operation::{Insert, InsertSource, Operation};
use crate::program::Program;
use crate::target::TargetKind;
use dol_expr::expr::InsertNode;
use dol_query::InsertQuery;

use super::BuildError;

/// Build the arena-based IR as a [`Program`] containing a single
/// [`Operation::Insert`] referencing an arena `ExprNode` of opcode
/// [`dol_expr::expr::ExprOp::Insert`].
///
/// Infallible in current shape (the builder only allocates `Param`
/// placeholders), but returns `Result` for API consistency with the
/// other lowering helpers. Will gain real failure modes once
/// user-supplied VALUES expressions are supported.
// budget-gate: opt-out: top-level entry point; constructs no `Budget`
// today (the current lowering is non-recursive Param allocation), but
// the surface signature still matches the gate's `pub fn lower*` regex.
pub fn lower_insert(query: InsertQuery) -> Result<Program, BuildError> {
    let mut arena = dol_expr::ExprArena::new();
    let mut interner = dol_expr::Interner::new();

    let fields = if query.fields.is_empty() {
        query.field_names.clone().unwrap_or_default()
    } else {
        query.fields.clone()
    };

    let target_str = interner.intern(&dol_expr::lower::qualified_name(
        &query.name,
        &query.namespace,
    ));
    let columns: smallvec::SmallVec<[dol_expr::ids::StrId; 8]> =
        fields.iter().map(|f| interner.intern(f)).collect();

    // Generate one Param node per field per row.
    let mut values: smallvec::SmallVec<[dol_expr::ids::NodeId; 8]> = smallvec::SmallVec::new();
    // `row_count * fields.len()` is bounded by the same constraints as
    // `param_count`; not representable as overflow on 64-bit `usize`.
    #[allow(clippy::arithmetic_side_effects)]
    for _ in 0..(query.row_count * fields.len()) {
        values.push(arena.alloc_param());
    }

    // Returning columns as field-reference expressions.
    let returning: smallvec::SmallVec<[dol_expr::ids::NodeId; 4]> = query
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

    let inode = InsertNode {
        target: target_str,
        columns,
        values,
        returning,
        conflict: None,
    };
    let iid = arena.alloc_insert(inode);
    let body = arena.alloc_insert_ref(iid);

    let target = crate::builders::target::target_from_parts(
        &mut interner,
        TargetKind::Relation,
        &query.name,
        query.namespace.as_deref(),
    );
    let op: Operation = Insert {
        target,
        source: InsertSource::Node(body),
        returning: None,
    }
    .into();
    Ok(Program::new(op, arena, interner))
}
