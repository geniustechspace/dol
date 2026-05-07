//! Lower a [`dol_query::UpdateQuery`] into an [`Operation::Update`] program.

extern crate alloc;
use alloc::string::ToString;

use crate::operation::{Operation, Update};
use crate::program::Program;
use crate::target::TargetKind;
use dol_core::policy::{Budget, Limits};
use dol_expr::expr::UpdateNode;
use dol_expr::lower::{lower_expr_with_budget, lower_filters};
use dol_query::UpdateQuery;

use super::BuildError;

/// Build the arena-based IR as a [`Program`] containing a single
/// [`Operation::Update`] referencing an arena `ExprNode` of opcode
/// [`dol_expr::expr::ExprOp::Update`].
///
/// Fallible: returns [`BuildError::SetValue`] / [`BuildError::Filter`]
/// when lowering an assignment RHS or the WHERE clause exhausts the
/// default budget.
// budget-gate: opt-out: this is the public entry point that *constructs*
// the `Budget`; downstream calls to `lower_expr_with_budget` /
// `lower_filters` thread it. The gate's pattern matcher only sees the
// surface signature, hence the opt-out.
pub fn lower_update(query: UpdateQuery) -> Result<Program, BuildError> {
    let mut arena = dol_expr::ExprArena::new();
    let mut interner = dol_expr::Interner::new();
    let mut budget = Budget::new(Limits::host());

    let target_str = interner.intern(&dol_expr::lower::qualified_name(
        &query.name,
        &query.namespace,
    ));

    let mut columns: smallvec::SmallVec<[dol_expr::ids::StrId; 8]> = smallvec::SmallVec::new();
    let mut values: smallvec::SmallVec<[dol_expr::ids::NodeId; 8]> = smallvec::SmallVec::new();
    for (col, expr) in &query.assignments {
        columns.push(interner.intern(col));
        let nid = lower_expr_with_budget(expr, &mut arena, &mut interner, &mut budget).map_err(
            |cause| BuildError::SetValue {
                column: col.to_string(),
                cause,
            },
        )?;
        values.push(nid);
    }

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

    let unode = UpdateNode {
        target: target_str,
        columns,
        values,
        filter,
        returning,
    };
    let uid = arena.alloc_update(unode);
    let body = arena.alloc_update_ref(uid);

    let target = crate::builders::target::target_from_parts(
        &mut interner,
        TargetKind::Relation,
        &query.name,
        query.namespace.as_deref(),
    );
    let op: Operation = Update { target, node: body }.into();
    Ok(Program::new(op, arena, interner))
}
