//! Lower a [`dol_query::GetQuery`] into an [`Operation::Query`] program.
//!
//! Mirrors what `GetQuery::try_build` used to do in the dol-query crate.

extern crate alloc;
use alloc::format;
use alloc::vec::Vec;

use crate::operation::{Operation, Query as OpQuery};
use crate::program::Program;
use crate::target::TargetKind;
use dol_core::policy::{Budget, Limits};
use dol_expr::expr::{JoinNode, JoinType as ArenaJoinType, QueryNode};
use dol_expr::ids::NodeId;
use dol_expr::lower::{lower_expr_with_budget, lower_exprs, lower_filters, lower_order_by};
use dol_expr::tree::{Expr, field_dyn};
use dol_query::{GetQuery, JoinKind};
use smallvec::SmallVec;

use super::BuildError;

/// Consume the builder and produce a [`Program`] holding a single
/// [`Operation::Query`] that references an arena `ExprNode` of opcode
/// [`dol_expr::expr::ExprOp::Query`] carrying the SELECT body.
///
/// When no projections have been set and `field_names` is available,
/// all listed fields are selected by default.
///
/// Fallible: returns the matching [`BuildError`] variant when lowering
/// any of the projection / WHERE / GROUP BY / HAVING / ORDER BY
/// expressions exhausts the default budget.
// budget-gate: opt-out: this is the public entry point that *constructs*
// the `Budget`; downstream calls to `lower_exprs` / `lower_filters` /
// `lower_expr_with_budget` / `lower_order_by` thread it, and the loop
// over JOIN ON conditions ticks the same budget. The gate's pattern
// matcher only sees the surface signature, hence the opt-out.
pub fn lower_get(query: GetQuery) -> Result<Program, BuildError> {
    let mut arena = dol_expr::ExprArena::new();
    let mut interner = dol_expr::Interner::new();
    let mut budget = Budget::new(Limits::host());

    let from = interner.intern(&dol_expr::lower::qualified_name(
        &query.name,
        &query.namespace,
    ));
    let alias = query.table_alias.as_deref().map(|a| interner.intern(a));

    // Default projections.
    let proj_exprs: Vec<Expr<'static>> = if query.projections.is_empty() {
        if let Some(ref names) = query.field_names {
            names.iter().map(|n| field_dyn(n)).collect()
        } else {
            Vec::new()
        }
    } else {
        query.projections.clone()
    };
    let columns: SmallVec<[NodeId; 8]> =
        lower_exprs(&proj_exprs, &mut arena, &mut interner, &mut budget)
            .map_err(BuildError::Projection)?;

    // Joins.
    let mut joins: SmallVec<[JoinNode; 2]> = SmallVec::new();
    for jc in &query.joins {
        // Honor the parsed `"namespace.name"` form by re-joining the
        // dotted source. The interner deduplicates so this is cheap;
        // backends parse the dotted form when they need the parts.
        let source = match &jc.target_namespace {
            Some(ns) => interner.intern(&format!("{ns}.{}", jc.target_name)),
            None => interner.intern(&jc.target_name),
        };
        let alias = jc.alias.as_deref().map(|a| interner.intern(a));
        let join_type = match jc.join_type {
            JoinKind::Inner => ArenaJoinType::Inner,
            JoinKind::Left => ArenaJoinType::Left,
            JoinKind::Right => ArenaJoinType::Right,
            JoinKind::Full => ArenaJoinType::Full,
            JoinKind::Cross => ArenaJoinType::Cross,
        };
        // Build ON condition from pairs (`None` for absent, e.g.
        // `CROSS JOIN`).
        let on: Option<NodeId> = if jc.on_conditions.is_empty() {
            None
        } else {
            // ON-conditions are simple `col = col` pairs and don't
            // need budget-aware lowering — they're constructed
            // structurally from interner ids. We still charge the
            // budget at allocation so a 4 G `ON ... AND ...` chain
            // can't run unbounded.
            let mut cond_ids: Vec<NodeId> = Vec::new();
            for (l, r) in &jc.on_conditions {
                // Charge two ticks: one for each `Field` allocation
                // (left and right column references) emitted below.
                budget
                    .tick(2)
                    .map_err(|e| BuildError::JoinOn(dol_expr::lower::LowerError::from(e)))?;
                let lid = {
                    let col = interner.intern(l);
                    let fid = arena.alloc_field(dol_expr::FieldNode {
                        namespace: None,
                        name: col,
                        steps: SmallVec::new(),
                    });
                    arena.alloc_field_ref(fid)
                };
                let rid = {
                    let col = interner.intern(r);
                    let fid = arena.alloc_field(dol_expr::FieldNode {
                        namespace: None,
                        name: col,
                        steps: SmallVec::new(),
                    });
                    arena.alloc_field_ref(fid)
                };
                cond_ids.push(arena.alloc_bin(dol_expr::expr::BinOp::Eq, lid, rid));
            }
            // `on_conditions.is_empty()` was checked above, so the
            // loop ran at least once and `cond_ids` is non-empty.
            #[allow(clippy::indexing_slicing)]
            let mut result = cond_ids[0];
            #[allow(clippy::indexing_slicing)]
            for id in &cond_ids[1..] {
                result = arena.alloc_bin(dol_expr::expr::BinOp::And, result, *id);
            }
            Some(result)
        };
        joins.push(JoinNode {
            source,
            alias,
            join_type,
            on,
        });
    }

    let filter = lower_filters(&query.filters, &mut arena, &mut interner, &mut budget)
        .map_err(BuildError::Filter)?;

    let mut group_by: SmallVec<[NodeId; 4]> = SmallVec::new();
    for e in &query.group_by {
        let nid = lower_expr_with_budget(e, &mut arena, &mut interner, &mut budget)
            .map_err(BuildError::GroupBy)?;
        group_by.push(nid);
    }

    let having: Option<NodeId> = if query.having.is_empty() {
        None
    } else {
        lower_filters(&query.having, &mut arena, &mut interner, &mut budget)
            .map_err(BuildError::Having)?
    };

    let mut order_by: SmallVec<[(NodeId, dol_expr::expr::Order); 4]> = SmallVec::new();
    for ob in &query.order_by {
        let pair = lower_order_by(ob, &mut arena, &mut interner, &mut budget)
            .map_err(BuildError::OrderBy)?;
        order_by.push(pair);
    }

    // Honour `query.lock_mode` when present; the `sql` feature gate now
    // lives entirely on `dol-query`, so `lock_mode` is `Option<LockHint>`
    // unconditionally on this side.
    let lock = query.lock_mode;

    let qnode = QueryNode {
        from,
        alias,
        joins,
        filter,
        columns,
        group_by,
        having,
        order_by,
        limit: None,
        offset: None,
        lock,
    };

    // Lower the QueryNode into the arena and reference it from
    // Operation::Query.
    let qid = arena.alloc_query(qnode);
    let body = arena.alloc_query_ref(qid);

    let target = crate::builders::target::target_from_parts(
        &mut interner,
        TargetKind::Relation,
        &query.name,
        query.namespace.as_deref(),
    );
    let op: Operation = OpQuery {
        target,
        node: Some(body),
    }
    .into();
    Ok(Program::new(op, arena, interner))
}
