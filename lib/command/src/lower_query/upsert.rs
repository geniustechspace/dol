//! Lower a [`dol_query::UpsertQuery`] into an [`Operation::Upsert`] program.

extern crate alloc;

use crate::operation::{Operation, Upsert};
use crate::program::Program;
use crate::target::TargetKind;
use dol_expr::expr::{ConflictClause, UpsertNode};
use dol_query::UpsertQuery;

use super::BuildError;

/// Build the arena-based IR as a [`Program`] containing a single
/// [`Operation::Upsert`] referencing an arena `ExprNode` of opcode
/// [`dol_expr::expr::ExprOp::Upsert`].
///
/// Infallible in current shape (the builder only emits `Param`
/// placeholders and structural `EXCLUDED.col` field references), but
/// returns `Result` for API consistency with the other lowering helpers.
// budget-gate: opt-out: top-level entry point; constructs no `Budget`
// today (the current lowering is non-recursive Param allocation + a
// fixed-shape conflict clause), but the surface signature still matches
// the gate's `pub fn lower*` regex.
pub fn lower_upsert(query: UpsertQuery) -> Result<Program, BuildError> {
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

    // One Param per field.
    let values: smallvec::SmallVec<[dol_expr::ids::NodeId; 8]> =
        fields.iter().map(|_| arena.alloc_param()).collect();

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

    let conflict = if query.then_skip_flag {
        Some(ConflictClause::DoNothing)
    } else if !query.update_fields.is_empty() {
        let assignments: smallvec::SmallVec<[(dol_expr::ids::StrId, dol_expr::ids::NodeId); 4]> =
            query
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

    let target = crate::builders::target::target_from_parts(
        &mut interner,
        TargetKind::Relation,
        &query.name,
        query.namespace.as_deref(),
    );
    let op: Operation = Upsert { target, node: body }.into();
    Ok(Program::new(op, arena, interner))
}
