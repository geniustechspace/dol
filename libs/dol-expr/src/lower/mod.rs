//! Lowering bridge: converts `crate::tree::Expr<'static>` into the
//! arena-based representation (`ExprArena` + `NodeId`).
//!
//! This module bridges the gap between the tree-based expression AST (with
//! lifetime-parameterised `Expr<'a>`) and the arena-based IR used by
//! `dol-ir::Statement`.

use crate::arena::{ExprArena, FieldNode, FuncNode, InListNode, ObjLitNode};
use crate::expr::{BinOp, ExprNode, Order, UnaryOp as ArenaUnaryOp};
use crate::ids::{NULL_NODE, NodeId};
use crate::interner::Interner;
use crate::tree::{Direction, Expr, OrderByExpr};
use smallvec::SmallVec;

/// Lowers an `Expr<'static>` into the arena, returning the root `NodeId`.
///
/// All strings are interned into `interner`.  Sub-expressions are recursively
/// lowered in post-order.
pub fn lower_expr(expr: &Expr<'_>, arena: &mut ExprArena, interner: &mut Interner) -> NodeId {
    use crate::arena::FieldStep;

    match expr {
        Expr::Namespace(path) => {
            // A bare container address — interned as its dotted form.
            let dotted = path.iter().collect::<Vec<_>>().join(".");
            let id = interner.intern(&dotted);
            arena.alloc(ExprNode::Namespace(id))
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
                Some(_other) => {
                    // The tree-level constructors never produce a non-Namespace
                    // base for `Expr::Field`. If a future shape introduces one
                    // (e.g. parameter-anchored leaves), lowering must be
                    // extended explicitly; until then, surface the violation
                    // loudly in debug builds and fall back to an unanchored
                    // leaf in release builds.
                    debug_assert!(
                        false,
                        "Expr::Field base must be Some(Expr::Namespace(_)) or None"
                    );
                    None
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
            arena.alloc(ExprNode::Field(fid))
        }

        Expr::Param => arena.alloc(ExprNode::Param),

        Expr::Value(lit) => {
            // dol-expr::Literal — clone directly.
            let lid = arena.alloc_lit(lit.clone().into_static());
            arena.alloc(ExprNode::Lit(lid))
        }

        Expr::Array(elements) => {
            let ids: SmallVec<[NodeId; 4]> = elements
                .iter()
                .map(|e| lower_expr(e, arena, interner))
                .collect();
            arena.alloc(ExprNode::ArrayLit(ids))
        }

        Expr::Object(fields) => {
            let pairs: SmallVec<[(u32, NodeId); 4]> = fields
                .iter()
                .map(|(k, v)| {
                    let kid = interner.intern(k.as_str());
                    let vid = lower_expr(v, arena, interner);
                    (kid, vid)
                })
                .collect();
            let oid = arena.alloc_obj_lit(ObjLitNode(pairs));
            arena.alloc(ExprNode::ObjectLit(oid))
        }

        Expr::BinaryOp { left, op, right } => {
            let lhs = lower_expr(left, arena, interner);
            let rhs = lower_expr(right, arena, interner);
            let bin_op = lower_binop(op);
            arena.alloc(ExprNode::BinOp {
                op: bin_op,
                lhs,
                rhs,
            })
        }

        Expr::UnaryOp { op, expr: inner } => {
            use crate::tree::UnaryOp as CoreUnaryOp;

            // Canonicalise NOT(InList) to a separate InList node.
            if *op == CoreUnaryOp::Not {
                if let Expr::InList { expr: ie, list } = inner.as_ref() {
                    // Lower as NOT + InList.
                    let in_node = lower_expr(
                        &Expr::InList {
                            expr: ie.clone(),
                            list: list.clone(),
                        },
                        arena,
                        interner,
                    );
                    return arena.alloc(ExprNode::UnaryOp {
                        op: ArenaUnaryOp::Not,
                        operand: in_node,
                    });
                }
            }

            let inner_id = lower_expr(inner, arena, interner);
            let arena_op = match op {
                CoreUnaryOp::Not => ArenaUnaryOp::Not,
                CoreUnaryOp::Neg => ArenaUnaryOp::Neg,
                CoreUnaryOp::IsNull => ArenaUnaryOp::IsNull,
                CoreUnaryOp::IsNotNull => ArenaUnaryOp::IsNotNull,
                CoreUnaryOp::BitNot => ArenaUnaryOp::Not, // dol-expr has no BitNot; approximate as Not
            };
            arena.alloc(ExprNode::UnaryOp {
                op: arena_op,
                operand: inner_id,
            })
        }

        Expr::Func { name, args } => {
            let func_name = interner.intern(name.name());
            let arg_ids: SmallVec<[NodeId; 4]> = args
                .iter()
                .map(|a| lower_expr(a, arena, interner))
                .collect();
            let fid = arena.alloc_func(FuncNode {
                name: func_name,
                args: arg_ids,
            });
            arena.alloc(ExprNode::Func(fid))
        }

        Expr::Cast {
            expr: inner,
            as_type,
        } => {
            let inner_id = lower_expr(inner, arena, interner);
            let type_name = interner.intern(&format!("{:?}", as_type));
            arena.alloc(ExprNode::Cast {
                expr: inner_id,
                to: type_name,
            })
        }

        Expr::Case { whens, else_expr } => {
            let branches: SmallVec<[(NodeId, NodeId); 4]> = whens
                .iter()
                .map(|(c, t)| {
                    (
                        lower_expr(c, arena, interner),
                        lower_expr(t, arena, interner),
                    )
                })
                .collect();
            let else_id = else_expr
                .as_deref()
                .map(|e| lower_expr(e, arena, interner))
                .unwrap_or(NULL_NODE);
            let cid = arena.alloc_case(crate::CaseNode {
                branches,
                else_: else_id,
            });
            arena.alloc(ExprNode::Case(cid))
        }

        Expr::Between {
            expr: inner,
            low,
            high,
        } => {
            let eid = lower_expr(inner, arena, interner);
            let lo = lower_expr(low, arena, interner);
            let hi = lower_expr(high, arena, interner);
            arena.alloc(ExprNode::Between { expr: eid, lo, hi })
        }

        Expr::InList { expr: inner, list } => {
            let eid = lower_expr(inner, arena, interner);
            let ids: SmallVec<[NodeId; 8]> = list
                .iter()
                .map(|e| lower_expr(e, arena, interner))
                .collect();
            let in_id = arena.alloc_in_list(InListNode {
                expr: eid,
                list: ids,
            });
            arena.alloc(ExprNode::InList(in_id))
        }

        Expr::Alias { expr: inner, alias } => {
            let inner_id = lower_expr(inner, arena, interner);
            let aid = interner.intern(alias.as_str());
            arena.alloc(ExprNode::Alias {
                expr: inner_id,
                name: aid,
            })
        }

        Expr::Star => {
            let col = interner.intern("*");
            let fid = arena.alloc_field(FieldNode {
                namespace: None,
                name: col,
                steps: SmallVec::new(),
            });
            arena.alloc(ExprNode::Field(fid))
        }

        Expr::CountStar => {
            let func_name = interner.intern("count");
            let star_col = interner.intern("*");
            let star_fid = arena.alloc_field(FieldNode {
                namespace: None,
                name: star_col,
                steps: SmallVec::new(),
            });
            let star_node = arena.alloc(ExprNode::Field(star_fid));
            arena.alloc(ExprNode::Agg {
                func: func_name,
                expr: star_node,
                distinct: false,
            })
        }

        Expr::Window {
            func,
            partition_by,
            order_by,
            ..
        } => {
            let func_name = match func.as_ref() {
                Expr::Func { name, .. } => interner.intern(name.name()),
                _ => interner.intern("unknown"),
            };
            let partition: SmallVec<[NodeId; 4]> = partition_by
                .iter()
                .map(|e| lower_expr(e, arena, interner))
                .collect();
            let order: SmallVec<[(NodeId, Order); 2]> = order_by
                .iter()
                .map(|ob| {
                    let eid = lower_expr(&ob.expr, arena, interner);
                    let dir = match ob.direction {
                        Direction::Asc => Order::Asc,
                        Direction::Desc => Order::Desc,
                    };
                    (eid, dir)
                })
                .collect();
            let wid = arena.alloc_window(crate::WindowNode {
                func: func_name,
                partition,
                order,
            });
            arena.alloc(ExprNode::Window(wid))
        }
    }
}

/// Lower an `OrderByExpr<'static>` into the arena, returning `(NodeId, Order)`.
pub fn lower_order_by(
    ob: &OrderByExpr<'_>,
    arena: &mut ExprArena,
    interner: &mut Interner,
) -> (NodeId, Order) {
    let nid = lower_expr(&ob.expr, arena, interner);
    let dir = match ob.direction {
        Direction::Asc => Order::Asc,
        Direction::Desc => Order::Desc,
    };
    (nid, dir)
}

/// Lower multiple expressions, returning `NodeId`s.
pub fn lower_exprs(
    exprs: &[Expr<'_>],
    arena: &mut ExprArena,
    interner: &mut Interner,
) -> SmallVec<[NodeId; 8]> {
    exprs
        .iter()
        .map(|e| lower_expr(e, arena, interner))
        .collect()
}

/// Format a qualified entity name (e.g. "namespace.name" or just "name").
pub fn qualified_name(name: &str, namespace: &Option<String>) -> String {
    match namespace {
        Some(ns) => format!("{}.{}", ns, name),
        None => name.to_string(),
    }
}

/// Lower multiple expressions and AND-join them, returning a single filter
/// `NodeId` (or `NULL_NODE` if the list is empty).
pub fn lower_filters(
    filters: &[Expr<'_>],
    arena: &mut ExprArena,
    interner: &mut Interner,
) -> NodeId {
    if filters.is_empty() {
        return NULL_NODE;
    }
    let mut ids: Vec<NodeId> = filters
        .iter()
        .map(|e| lower_expr(e, arena, interner))
        .collect();
    let mut result = ids.remove(0);
    for id in ids {
        result = arena.alloc(ExprNode::BinOp {
            op: BinOp::And,
            lhs: result,
            rhs: id,
        });
    }
    result
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
    use crate::expr::ExprNode;
    use crate::tree::{field, namespace};

    #[test]
    fn lowers_bare_field_to_unanchored_arena_field() {
        let mut arena = ExprArena::new();
        let mut interner = Interner::new();
        let id = lower_expr(&field("email"), &mut arena, &mut interner);
        match arena.get(id) {
            ExprNode::Field(fid) => {
                let node = arena.get_field(*fid);
                assert!(node.namespace.is_none());
                assert_eq!(interner.get(node.name), "email");
                assert!(node.steps.is_empty());
            }
            other => panic!("expected ExprNode::Field, got {:?}", other),
        }
    }

    #[test]
    fn lowers_namespace_anchored_field() {
        let mut arena = ExprArena::new();
        let mut interner = Interner::new();
        let expr = namespace("users").field("email");
        let id = lower_expr(&expr, &mut arena, &mut interner);
        match arena.get(id) {
            ExprNode::Field(fid) => {
                let node = arena.get_field(*fid);
                let ns = node.namespace.expect("anchored on namespace");
                assert_eq!(interner.get(ns), "users");
                assert_eq!(interner.get(node.name), "email");
            }
            other => panic!("expected ExprNode::Field, got {:?}", other),
        }
    }

    #[test]
    fn lowers_namespace_only_path_to_namespace_node() {
        let mut arena = ExprArena::new();
        let mut interner = Interner::new();
        let id = lower_expr(&namespace("schema.users"), &mut arena, &mut interner);
        match arena.get(id) {
            ExprNode::Namespace(sid) => {
                assert_eq!(interner.get(*sid), "schema.users");
            }
            other => panic!("expected ExprNode::Namespace, got {:?}", other),
        }
    }

    #[test]
    fn lowers_in_leaf_traversal_steps() {
        use crate::arena::FieldStep;
        let mut arena = ExprArena::new();
        let mut interner = Interner::new();
        let expr = field("profile").get("address").get("city");
        let id = lower_expr(&expr, &mut arena, &mut interner);
        match arena.get(id) {
            ExprNode::Field(fid) => {
                let node = arena.get_field(*fid);
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
            other => panic!("expected ExprNode::Field, got {:?}", other),
        }
    }
}
