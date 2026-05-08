//! Lower a [`dol_query::DeleteQuery`] into an [`Operation::Delete`] program.
//!
//! Mirrors what `DeleteQuery::try_build` used to do in the dol-query
//! crate; the move inverts the historical `dol-query → dol-command` edge.

extern crate alloc;

use crate::operation::{Delete, Operation};
use crate::program::Program;
use crate::target::TargetKind;
use dol_core::policy::{Budget, Limits};
use dol_expr::expr::DeleteNode;
use dol_expr::lower::lower_filters;
use dol_query::DeleteQuery;

use super::BuildError;

/// Build the arena-based IR as a [`Program`] containing a single
/// [`Operation::Delete`] referencing an arena `ExprNode` of opcode
/// [`dol_expr::expr::ExprOp::Delete`].
///
/// Fallible: returns [`BuildError::Filter`] when lowering the WHERE
/// clause exhausts the default [`Budget`] (depth or fuel cap from
/// [`Limits::host`]).
// budget-gate: opt-out: this is the public entry point that *constructs*
// the `Budget`; downstream calls to `lower_filters` thread it. The gate's
// pattern matcher only sees the surface signature, hence the opt-out.
pub fn lower_delete(query: DeleteQuery) -> Result<Program, BuildError> {
    let mut arena = dol_expr::ExprArena::new();
    let mut interner = dol_expr::Interner::new();
    let mut budget = Budget::new(Limits::host());

    let target_str = interner.intern(&dol_expr::lower::qualified_name(
        &query.name,
        &query.namespace,
    ));
    let filter = lower_filters(&query.filters, &mut arena, &mut interner, &mut budget)
        .map_err(BuildError::Filter)?;

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

    let dnode = DeleteNode {
        target: target_str,
        filter,
        returning,
    };
    let did = arena.alloc_delete(dnode);
    let body = arena.alloc_delete_ref(did);

    let target = crate::builders::target::target_from_parts(
        &mut interner,
        TargetKind::Relation,
        &query.name,
        query.namespace.as_deref(),
    );
    let op: Operation = Delete { target, node: body }.into();
    Ok(Program::new(op, arena, interner))
}
