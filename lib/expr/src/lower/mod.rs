//! Lowering bridge: converts `crate::tree::Expr<'static>` into the
//! arena-based representation (`ExprArena` + `NodeId`).
//!
//! This module bridges the gap between the tree-based expression AST (with
//! lifetime-parameterised `Expr<'a>`) and the arena-based IR used by
//! `dol-ir::Operation`.
//!
//! Lowering is fallible: shapes that have no faithful arena representation
//! (e.g. `Window` over a non-`Func` head) surface as a [`LowerError`] rather
//! than silently degrading.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use crate::arena::{ArrayLitNode, ExprArena, FieldNode, FuncNode, InListNode, ObjLitNode};
use crate::expr::{BinOp, Order, UnaryOp as ArenaUnaryOp};
use crate::ids::NodeId;
use crate::interner::Interner;
use crate::tree::{Direction, Expr, OrderByExpr};
use dol_core::policy::{Budget, Limits};
use smallvec::SmallVec;

/// Errors produced by the tree → arena lowering pipeline.
///
/// Variants are `#[non_exhaustive]` so new lossy paths can be surfaced
/// without breaking downstream match arms.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LowerError {
    /// `Expr::Window` was lowered with a head that is not `Expr::Func`.
    /// The arena representation only models named window-function heads;
    /// faithfully lowering anything else would require a different shape,
    /// so we refuse rather than guess.
    UnsupportedWindowHead {
        /// Free-form description of what was found in the head position.
        head: String,
    },
    /// A field reference's `base` was not `None` or `Expr::Namespace(_)`.
    /// The tree-level constructors guarantee one of those two shapes; this
    /// only triggers if a future shape introduces a new base form without
    /// extending lowering.
    UnsupportedFieldBase {
        /// Free-form description of the rejected base shape.
        base: String,
    },
    /// The traversal `Budget` ran out of node fuel mid-lowering — the
    /// input `Expr<'_>` produced more arena allocations than the
    /// configured [`Limits::max_nodes`]. Raise the limit on the
    /// [`crate::BuildSession`] (or pass a more generous `Budget` to
    /// [`lower_expr_with_budget`]) or simplify the source.
    FuelExhausted,
    /// The recursion / tree depth exceeded the traversal's configured
    /// [`Limits::max_depth`]. Protects against pathologically deep
    /// expressions (deep `OR`-chains, untrusted IR) that would
    /// otherwise overflow the host stack.
    DepthExceeded {
        /// Depth at which the limit was hit.
        depth: u32,
    },
}

impl core::fmt::Display for LowerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LowerError::UnsupportedWindowHead { head } => {
                write!(f, "unsupported window head: {head}")
            }
            LowerError::UnsupportedFieldBase { base } => {
                write!(f, "unsupported field base: {base}")
            }
            LowerError::FuelExhausted => f.write_str("lowering fuel exhausted"),
            LowerError::DepthExceeded { depth } => {
                write!(f, "lowering depth limit exceeded at depth {depth}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LowerError {}

impl From<dol_core::policy::BudgetError> for LowerError {
    fn from(e: dol_core::policy::BudgetError) -> Self {
        match e {
            dol_core::policy::BudgetError::Depth => LowerError::DepthExceeded { depth: 0 },
            dol_core::policy::BudgetError::Nodes
            | dol_core::policy::BudgetError::Bytes
            | dol_core::policy::BudgetError::StrBytes => LowerError::FuelExhausted,
        }
    }
}

/// Lowers an `Expr<'static>` into the arena, returning the root `NodeId`.
///
/// All strings are interned into `interner`. Sub-expressions are recursively
/// lowered in post-order. Returns [`LowerError`] when an expression shape
/// has no faithful arena representation; callers should propagate the
/// diagnostic rather than swallow it.
///
/// **Unbounded:** this entry point uses [`Limits::unbounded`], so it
/// performs no effective fuel / depth checking. For untrusted or
/// adversarial input, prefer [`crate::BuildSession::lower`] (or call
/// [`lower_expr_with_budget`] directly with a tighter [`Budget`]) so
/// deeply-nested expressions or runaway allocation are rejected with a
/// `LowerError` rather than blowing the stack or the heap.
// budget-gate: opt-out: documented unbounded convenience entry point;
// the bounded sibling `lower_expr_with_budget` is the production path.
pub fn lower_expr(
    expr: &Expr<'_>,
    arena: &mut ExprArena,
    interner: &mut Interner,
) -> Result<NodeId, LowerError> {
    // The shared bounded implementation pays one branch per allocation
    // even on this path, but the cost is negligible compared to the
    // allocation itself, and consolidating keeps both entry points
    // byte-identical in observable behaviour.
    let mut budget = Budget::new(Limits::unbounded());
    lower_expr_with_budget(expr, arena, interner, &mut budget)
}

/// Bounded counterpart to [`lower_expr`].
///
/// Threads a [`Budget`] through the recursion: each node charges one
/// tick (mapping [`dol_core::policy::BudgetError::Nodes`] →
/// [`LowerError::FuelExhausted`]) and each recursive descent calls
/// [`Budget::descend`] (mapping
/// [`dol_core::policy::BudgetError::Depth`] →
/// [`LowerError::DepthExceeded`] with `depth = budget.limits().max_depth`,
/// the cap that was hit).
///
/// The supplied `budget` keeps accumulating across calls — node ticks
/// are not refunded between separate `lower_expr_with_budget`
/// invocations on the same `Budget`. This is intentional so that
/// [`crate::BuildSession`] can enforce a quota across the whole session
/// rather than per `lower()` call.
pub fn lower_expr_with_budget(
    expr: &Expr<'_>,
    arena: &mut ExprArena,
    interner: &mut Interner,
    budget: &mut Budget,
) -> Result<NodeId, LowerError> {
    lower_child(expr, arena, interner, budget)
}

/// Lower one tree node *as a child* of an enclosing context: descends
/// the budget by one level, runs `lower_inner`, and on either fuel- or
/// depth-exhaustion produces the matching [`LowerError`].
///
/// All recursive lowering ultimately routes through here so the budget
/// machinery lives in one place.
fn lower_child(
    expr: &Expr<'_>,
    arena: &mut ExprArena,
    interner: &mut Interner,
    budget: &mut Budget,
) -> Result<NodeId, LowerError> {
    let max_depth = budget.limits().max_depth;
    match budget.descend(|b| lower_inner(expr, arena, interner, b)) {
        Ok(inner) => inner,
        Err(_) => Err(LowerError::DepthExceeded { depth: max_depth }),
    }
}

fn lower_inner(
    expr: &Expr<'_>,
    arena: &mut ExprArena,
    interner: &mut Interner,
    budget: &mut Budget,
) -> Result<NodeId, LowerError> {
    use crate::arena::FieldStep;

    // Each `lower_inner` invocation lowers one tree node, which always
    // allocates at least one arena node in every match arm below.
    // Charging here (instead of at every individual `arena.alloc` call
    // site) keeps the bounded path readable while still bounding total
    // allocations to within a small constant factor of `max_nodes`.
    budget.tick(1).map_err(|_| LowerError::FuelExhausted)?;

    match expr {
        Expr::Namespace(path) => {
            // A bare container address — interned as its dotted form.
            let dotted = path.iter().collect::<Vec<_>>().join(".");
            let id = interner.intern(&dotted);
            Ok(arena.alloc_namespace(id))
        }

        Expr::Field { base, name, steps } => {
            // Resolve the namespace anchor (if any) by interning its dotted
            // form. A non-namespace base is currently unsupported here — the
            // tree-level constructors only ever produce `Some(Namespace(_))`
            // or `None` for `base`.
            let namespace = match base.as_deref() {
                Some(Expr::Namespace(path)) => {
                    let dotted = path.iter().collect::<Vec<_>>().join(".");
                    Some(interner.intern(&dotted))
                }
                None => None,
                Some(other) => {
                    return Err(LowerError::UnsupportedFieldBase {
                        base: format!("{other:?}"),
                    });
                }
            };

            let leaf_id = interner.intern(name.as_str());
            let arena_steps: SmallVec<[FieldStep; 4]> = steps
                .iter()
                .map(|s| FieldStep::Key(interner.intern(s.as_str())))
                .collect();

            let fid = arena.alloc_field(FieldNode {
                namespace,
                name: leaf_id,
                steps: arena_steps,
            });
            Ok(arena.alloc_field_ref(fid))
        }

        Expr::Param => Ok(arena.alloc_param()),

        Expr::Value(lit) => {
            // dol-expr::Literal — clone directly.
            let lid = arena.alloc_lit(lit.clone().into_static());
            Ok(arena.alloc_lit_ref(lid))
        }

        Expr::Array(elements) => {
            let mut ids: SmallVec<[NodeId; 4]> = SmallVec::new();
            for e in elements {
                ids.push(lower_child(e, arena, interner, budget)?);
            }
            let aid = arena.alloc_array_lit(ArrayLitNode { items: ids });
            Ok(arena.alloc_array_lit_ref(aid))
        }

        Expr::Object(fields) => {
            let mut pairs: SmallVec<[(crate::ids::StrId, NodeId); 4]> = SmallVec::new();
            for (k, v) in fields {
                let kid = interner.intern(k.as_str());
                let vid = lower_child(v, arena, interner, budget)?;
                pairs.push((kid, vid));
            }
            let oid = arena.alloc_obj_lit(ObjLitNode(pairs));
            Ok(arena.alloc_object_lit_ref(oid))
        }

        Expr::BinaryOp { left, op, right } => {
            let lhs = lower_child(left, arena, interner, budget)?;
            let rhs = lower_child(right, arena, interner, budget)?;
            let bin_op = lower_binop(op);
            Ok(arena.alloc_bin(bin_op, lhs, rhs))
        }

        Expr::UnaryOp { op, expr: inner } => {
            use crate::tree::UnaryOp as CoreUnaryOp;

            // Canonicalise NOT(InList) to a separate InList node.
            if *op == CoreUnaryOp::Not {
                if let Expr::InList { expr: ie, list } = inner.as_ref() {
                    // Lower as NOT + InList.
                    let in_node = lower_child(
                        &Expr::InList {
                            expr: ie.clone(),
                            list: list.clone(),
                        },
                        arena,
                        interner,
                        budget,
                    )?;
                    return Ok(arena.alloc_una(ArenaUnaryOp::Not, in_node));
                }
            }

            let inner_id = lower_child(inner, arena, interner, budget)?;
            let arena_op = match op {
                CoreUnaryOp::Not => ArenaUnaryOp::Not,
                CoreUnaryOp::Neg => ArenaUnaryOp::Neg,
                CoreUnaryOp::IsNull => ArenaUnaryOp::IsNull,
                CoreUnaryOp::IsNotNull => ArenaUnaryOp::IsNotNull,
                CoreUnaryOp::BitNot => ArenaUnaryOp::BitNot,
            };
            Ok(arena.alloc_una(arena_op, inner_id))
        }

        Expr::Func { name, args } => {
            let func_name = interner.intern(name.name());
            let mut arg_ids: SmallVec<[NodeId; 4]> = SmallVec::new();
            for a in args {
                arg_ids.push(lower_child(a, arena, interner, budget)?);
            }
            let fid = arena.alloc_func(FuncNode {
                name: func_name,
                args: arg_ids,
            });
            Ok(arena.alloc_func_ref(fid))
        }

        Expr::Cast {
            expr: inner,
            as_type,
        } => {
            let inner_id = lower_child(inner, arena, interner, budget)?;
            // Use the stable `DataType::type_name()` rather than a `Debug`
            // rendering so the interned type name is wire-stable across
            // Rust toolchain upgrades.
            let type_name = interner.intern(as_type.type_name());
            Ok(arena.alloc_cast(inner_id, type_name))
        }

        Expr::Case { whens, else_expr } => {
            let mut branches: SmallVec<[(NodeId, NodeId); 4]> = SmallVec::new();
            for (c, t) in whens {
                let cid = lower_child(c, arena, interner, budget)?;
                let tid = lower_child(t, arena, interner, budget)?;
                branches.push((cid, tid));
            }
            let else_id = match else_expr.as_deref() {
                Some(e) => Some(lower_child(e, arena, interner, budget)?),
                None => None,
            };
            let cid = arena.alloc_case(crate::CaseNode {
                branches,
                else_: else_id,
            });
            Ok(arena.alloc_case_ref(cid))
        }

        Expr::Between {
            expr: inner,
            low,
            high,
        } => {
            let eid = lower_child(inner, arena, interner, budget)?;
            let lo = lower_child(low, arena, interner, budget)?;
            let hi = lower_child(high, arena, interner, budget)?;
            Ok(arena.alloc_between(eid, lo, hi))
        }

        Expr::InList { expr: inner, list } => {
            let eid = lower_child(inner, arena, interner, budget)?;
            let mut ids: SmallVec<[NodeId; 8]> = SmallVec::new();
            for e in list {
                ids.push(lower_child(e, arena, interner, budget)?);
            }
            let in_id = arena.alloc_in_list(InListNode {
                expr: eid,
                list: ids,
            });
            Ok(arena.alloc_in_list_ref(in_id))
        }

        Expr::Alias { expr: inner, alias } => {
            let inner_id = lower_child(inner, arena, interner, budget)?;
            let aid = interner.intern(alias.as_str());
            Ok(arena.alloc_alias(inner_id, aid))
        }

        Expr::Star => {
            let col = interner.intern("*");
            let fid = arena.alloc_field(FieldNode {
                namespace: None,
                name: col,
                steps: SmallVec::new(),
            });
            Ok(arena.alloc_field_ref(fid))
        }

        Expr::CountStar => {
            let func_name = interner.intern("count");
            let star_col = interner.intern("*");
            let star_fid = arena.alloc_field(FieldNode {
                namespace: None,
                name: star_col,
                steps: SmallVec::new(),
            });
            let star_node = arena.alloc_field_ref(star_fid);
            Ok(arena.alloc_agg(func_name, star_node, false))
        }

        Expr::Window {
            func,
            partition_by,
            order_by,
            frame,
        } => {
            // The arena's `WindowNode` only models named window-function
            // heads. A non-`Func` head has no faithful representation; we
            // refuse explicitly rather than interning a sentinel "unknown".
            let func_name = match func.as_ref() {
                Expr::Func { name, .. } => interner.intern(name.name()),
                other => {
                    return Err(LowerError::UnsupportedWindowHead {
                        head: format!("{other:?}"),
                    });
                }
            };
            let mut partition: SmallVec<[NodeId; 4]> = SmallVec::new();
            for e in partition_by {
                partition.push(lower_child(e, arena, interner, budget)?);
            }
            let mut order: SmallVec<[(NodeId, Order); 2]> = SmallVec::new();
            for ob in order_by {
                let eid = lower_child(&ob.expr, arena, interner, budget)?;
                let dir = match ob.direction {
                    Direction::Asc => Order::Asc,
                    Direction::Desc => Order::Desc,
                };
                order.push((eid, dir));
            }
            let wid = arena.alloc_window(crate::WindowNode {
                func: func_name,
                partition,
                order,
                frame: frame.clone(),
            });
            Ok(arena.alloc_window_ref(wid))
        }
    }
}

/// Lower an `OrderByExpr<'static>` into the arena, returning `(NodeId, Order)`.
///
/// Budget-aware. The supplied [`Budget`] is threaded into the inner
/// [`lower_expr_with_budget`] call.
pub fn lower_order_by(
    ob: &OrderByExpr<'_>,
    arena: &mut ExprArena,
    interner: &mut Interner,
    budget: &mut Budget,
) -> Result<(NodeId, Order), LowerError> {
    let nid = lower_expr_with_budget(&ob.expr, arena, interner, budget)?;
    let dir = match ob.direction {
        Direction::Asc => Order::Asc,
        Direction::Desc => Order::Desc,
    };
    Ok((nid, dir))
}

/// Lower multiple expressions, returning `NodeId`s.
///
/// Budget-aware. Each element charges through the shared
/// [`lower_expr_with_budget`] path.
pub fn lower_exprs(
    exprs: &[Expr<'_>],
    arena: &mut ExprArena,
    interner: &mut Interner,
    budget: &mut Budget,
) -> Result<SmallVec<[NodeId; 8]>, LowerError> {
    let mut out: SmallVec<[NodeId; 8]> = SmallVec::new();
    for e in exprs {
        out.push(lower_expr_with_budget(e, arena, interner, budget)?);
    }
    Ok(out)
}

/// Format a qualified entity name (e.g. "namespace.name" or just "name").
pub fn qualified_name(name: &str, namespace: &Option<String>) -> String {
    match namespace {
        Some(ns) => format!("{}.{}", ns, name),
        None => name.to_string(),
    }
}

/// Lower multiple expressions and AND-join them, returning a single filter
/// `NodeId` — or `None` if the list is empty (replaces the previous
/// `NULL_NODE` sentinel).
///
/// Budget-aware. Each element charges through the shared
/// [`lower_expr_with_budget`] path.
pub fn lower_filters(
    filters: &[Expr<'_>],
    arena: &mut ExprArena,
    interner: &mut Interner,
    budget: &mut Budget,
) -> Result<Option<NodeId>, LowerError> {
    if filters.is_empty() {
        return Ok(None);
    }
    let mut ids: Vec<NodeId> = Vec::with_capacity(filters.len());
    for f in filters {
        ids.push(lower_expr_with_budget(f, arena, interner, budget)?);
    }
    let mut result = ids.remove(0);
    for id in ids {
        result = arena.alloc_bin(BinOp::And, result, id);
    }
    Ok(Some(result))
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Map `crate::tree::OpDef` name to `dol-expr::expr::BinOp`.
fn lower_binop(op: &crate::tree::OpDef) -> BinOp {
    match op.name() {
        "EQ" => BinOp::Eq,
        "NE" => BinOp::Ne,
        "LT" => BinOp::Lt,
        "GT" => BinOp::Gt,
        "LE" => BinOp::Le,
        "GE" => BinOp::Ge,
        "ADD" => BinOp::Add,
        "SUB" => BinOp::Sub,
        "MUL" => BinOp::Mul,
        "DIV" => BinOp::Div,
        "MOD" => BinOp::Rem,
        "AND" => BinOp::And,
        "OR" => BinOp::Or,
        "LIKE" => BinOp::Like,
        "ILIKE" => BinOp::ILike,
        "SIMILAR_TO" => BinOp::Similar,
        "CONCAT" => BinOp::Concat,
        "BIT_AND" => BinOp::BitAnd,
        "BIT_OR" => BinOp::BitOr,
        "BIT_XOR" => BinOp::BitXor,
        "SHIFT_LEFT" => BinOp::Shl,
        "SHIFT_RIGHT" => BinOp::Shr,
        _ => BinOp::Eq, // Fallback for unknown operators.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{field, namespace};

    #[test]
    fn lowers_bare_field_to_unanchored_arena_field() {
        let mut arena = ExprArena::new();
        let mut interner = Interner::new();
        let id = lower_expr(&field("email"), &mut arena, &mut interner).unwrap();
        let fid = arena.get(id).as_field().expect("expected Field opcode");
        let node = arena.get_field(fid);
        assert!(node.namespace.is_none());
        assert_eq!(interner.get(node.name), "email");
        assert!(node.steps.is_empty());
    }

    #[test]
    fn lowers_namespace_anchored_field() {
        let mut arena = ExprArena::new();
        let mut interner = Interner::new();
        let expr = namespace("users").field("email");
        let id = lower_expr(&expr, &mut arena, &mut interner).unwrap();
        let fid = arena.get(id).as_field().expect("expected Field opcode");
        let node = arena.get_field(fid);
        let ns = node.namespace.expect("anchored on namespace");
        assert_eq!(interner.get(ns), "users");
        assert_eq!(interner.get(node.name), "email");
    }

    #[test]
    fn lowers_namespace_only_path_to_namespace_node() {
        let mut arena = ExprArena::new();
        let mut interner = Interner::new();
        let id = lower_expr(&namespace("schema.users"), &mut arena, &mut interner).unwrap();
        let sid = arena
            .get(id)
            .as_namespace()
            .expect("expected Namespace opcode");
        assert_eq!(interner.get(sid), "schema.users");
    }

    #[test]
    fn lowers_in_leaf_traversal_steps() {
        use crate::arena::FieldStep;
        let mut arena = ExprArena::new();
        let mut interner = Interner::new();
        let expr = field("profile").get("address").get("city");
        let id = lower_expr(&expr, &mut arena, &mut interner).unwrap();
        let fid = arena.get(id).as_field().expect("expected Field opcode");
        let node = arena.get_field(fid);
        assert_eq!(interner.get(node.name), "profile");
        assert_eq!(node.steps.len(), 2);
        let labels: Vec<&str> = node
            .steps
            .iter()
            .map(|s| match s {
                FieldStep::Key(id) => interner.get(*id),
                FieldStep::Index(_) => panic!("unexpected index step"),
            })
            .collect();
        assert_eq!(labels, ["address", "city"]);
    }

    #[test]
    fn lowers_bitnot_to_arena_bitnot() {
        use crate::tree::{UnaryOp as CoreUnaryOp, int};
        let mut arena = ExprArena::new();
        let mut interner = Interner::new();
        let inner = int(7i32);
        let expr = Expr::UnaryOp {
            op: CoreUnaryOp::BitNot,
            expr: alloc::boxed::Box::new(inner),
        };
        let id = lower_expr(&expr, &mut arena, &mut interner).unwrap();
        let (op, _) = arena.get(id).as_una().expect("expected Una opcode");
        assert_eq!(op, ArenaUnaryOp::BitNot);
    }

    #[test]
    fn lowers_cast_with_stable_type_name() {
        use crate::tree::int;
        use dol_core::DataType;
        let mut arena = ExprArena::new();
        let mut interner = Interner::new();
        let expr = Expr::Cast {
            expr: alloc::boxed::Box::new(int(7i32)),
            as_type: DataType::Int64,
        };
        let id = lower_expr(&expr, &mut arena, &mut interner).unwrap();
        let (_, to) = arena.get(id).as_cast().expect("expected Cast opcode");
        // `DataType::Int64` has the stable name `"int64"`.
        assert_eq!(interner.get(to), "int64");
    }

    #[test]
    fn rejects_window_with_non_func_head() {
        use crate::tree::field;
        let mut arena = ExprArena::new();
        let mut interner = Interner::new();
        let expr = Expr::Window {
            func: alloc::boxed::Box::new(field("x")),
            partition_by: alloc::vec::Vec::new(),
            order_by: alloc::vec::Vec::new(),
            frame: None,
        };
        let err = lower_expr(&expr, &mut arena, &mut interner).unwrap_err();
        assert!(matches!(err, LowerError::UnsupportedWindowHead { .. }));
    }

    // ── Bounded lowering — Budget ──────────────────────────────────────────

    #[test]
    fn bounded_lower_succeeds_with_ample_budget() {
        use crate::tree::int;
        let mut arena = ExprArena::new();
        let mut interner = Interner::new();
        let expr = field("a").eq(int(1i32));
        let mut budget = Budget::new(Limits {
            max_nodes: 100,
            max_depth: 100,
            max_bytes: 1 << 20,
            max_str_bytes: 1024,
        });
        let id = lower_expr_with_budget(&expr, &mut arena, &mut interner, &mut budget).unwrap();
        // At least one tick was charged (the BinOp itself).
        assert!(budget.nodes() > 0);
        // Depth fully restored after the call returns.
        assert_eq!(budget.depth(), 0);
        // And the node is a BinOp.
        assert!(arena.get(id).as_bin().is_some());
    }

    #[test]
    fn bounded_lower_rejects_exhausted_node_budget() {
        let mut arena = ExprArena::new();
        let mut interner = Interner::new();
        let expr = field("a");
        let mut budget = Budget::new(Limits {
            max_nodes: 0,
            max_depth: 32,
            max_bytes: 1 << 20,
            max_str_bytes: 1024,
        });
        let err =
            lower_expr_with_budget(&expr, &mut arena, &mut interner, &mut budget).unwrap_err();
        assert!(matches!(err, LowerError::FuelExhausted));
    }

    #[test]
    fn bounded_lower_rejects_overdeep_tree() {
        use crate::tree::int;
        // Build a left-heavy AND-chain of depth 8.
        let mut e = field("a").eq(int(0i32));
        for _ in 0..8 {
            e = e & field("b").eq(int(0i32));
        }
        let mut arena = ExprArena::new();
        let mut interner = Interner::new();
        // Cap at depth 4 so the chain is rejected.
        let mut budget = Budget::new(Limits {
            max_nodes: 1000,
            max_depth: 4,
            max_bytes: 1 << 20,
            max_str_bytes: 1024,
        });
        let err = lower_expr_with_budget(&e, &mut arena, &mut interner, &mut budget).unwrap_err();
        assert!(
            matches!(err, LowerError::DepthExceeded { depth } if depth == 4),
            "expected DepthExceeded {{ depth: 4 }}, got {err:?}"
        );
    }

    #[test]
    fn build_session_lower_threads_budget_through() {
        use crate::session::BuildSession;
        use crate::tree::int;
        let mut sess = BuildSession::new();
        // A two-arm AND has 5 arena nodes (field x2, lit x2, eq x2, and),
        // which exceeds max_nodes=2 and must surface as FuelExhausted.
        sess.set_limits(Limits {
            max_nodes: 2,
            max_depth: 512,
            max_bytes: 1 << 20,
            max_str_bytes: 1024,
        });
        let expr = field("a").eq(int(1i32)) & field("b").eq(int(2i32));
        let err = sess.lower(&expr).unwrap_err();
        assert!(matches!(err, LowerError::FuelExhausted));
    }

    /// Deep-OR-chain regression: lowering must reject via Budget
    /// (rather than blow the host stack) when the cap is set below the
    /// chain depth. The chain length here (`8 192`) is well above the
    /// typical SQL backend's nesting limit yet small enough to keep
    /// `Box<Expr>`'s recursive `Drop` within the default test stack —
    /// the production guard is the `Limits::max_depth` cap, not the
    /// host stack.
    #[test]
    fn deep_or_chain_does_not_overflow() {
        use crate::tree::int;
        let depth = 8_192usize;
        let mut e = field("a").eq(int(0i32));
        for _ in 0..depth {
            e = e | field("b").eq(int(0i32));
        }
        let mut arena = ExprArena::new();
        let mut interner = Interner::new();
        // Cap below the chain depth so the lowerer rejects with
        // DepthExceeded — no host-stack overflow even though the cap is
        // small enough to cut early.
        let mut budget = Budget::new(Limits {
            max_nodes: usize::MAX,
            max_depth: 128,
            max_bytes: usize::MAX,
            max_str_bytes: usize::MAX,
        });
        let result = lower_expr_with_budget(&e, &mut arena, &mut interner, &mut budget);
        assert!(matches!(result, Err(LowerError::DepthExceeded { .. })));
    }
}
