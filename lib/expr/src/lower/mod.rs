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

use crate::arena::{ExprArena, FieldNode, FuncNode, InListNode, ObjLitNode};
use crate::expr::{BinOp, ExprNode, Order, UnaryOp as ArenaUnaryOp};
use crate::ids::{NULL_NODE, NodeId};
use crate::interner::Interner;
use crate::tree::{Direction, Expr, OrderByExpr};
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
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LowerError {}

/// Lowers an `Expr<'static>` into the arena, returning the root `NodeId`.
///
/// All strings are interned into `interner`. Sub-expressions are recursively
/// lowered in post-order. Returns [`LowerError`] when an expression shape
/// has no faithful arena representation; callers should propagate the
/// diagnostic rather than swallow it.
pub fn lower_expr(
    expr: &Expr<'_>,
    arena: &mut ExprArena,
    interner: &mut Interner,
) -> Result<NodeId, LowerError> {
    use crate::arena::FieldStep;

    match expr {
        Expr::Namespace(path) => {
            // A bare container address — interned as its dotted form.
            let dotted = path.iter().collect::<Vec<_>>().join(".");
            let id = interner.intern(&dotted);
            Ok(arena.alloc(ExprNode::Namespace(id)))
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
            Ok(arena.alloc(ExprNode::Field(fid)))
        }

        Expr::Param => Ok(arena.alloc(ExprNode::Param)),

        Expr::Value(lit) => {
            // dol-expr::Literal — clone directly.
            let lid = arena.alloc_lit(lit.clone().into_static());
            Ok(arena.alloc(ExprNode::Lit(lid)))
        }

        Expr::Array(elements) => {
            let mut ids: SmallVec<[NodeId; 4]> = SmallVec::new();
            for e in elements {
                ids.push(lower_expr(e, arena, interner)?);
            }
            Ok(arena.alloc(ExprNode::ArrayLit(ids)))
        }

        Expr::Object(fields) => {
            let mut pairs: SmallVec<[(u32, NodeId); 4]> = SmallVec::new();
            for (k, v) in fields {
                let kid = interner.intern(k.as_str());
                let vid = lower_expr(v, arena, interner)?;
                pairs.push((kid, vid));
            }
            let oid = arena.alloc_obj_lit(ObjLitNode(pairs));
            Ok(arena.alloc(ExprNode::ObjectLit(oid)))
        }

        Expr::BinaryOp { left, op, right } => {
            let lhs = lower_expr(left, arena, interner)?;
            let rhs = lower_expr(right, arena, interner)?;
            let bin_op = lower_binop(op);
            Ok(arena.alloc(ExprNode::BinOp {
                op: bin_op,
                lhs,
                rhs,
            }))
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
                    )?;
                    return Ok(arena.alloc(ExprNode::UnaryOp {
                        op: ArenaUnaryOp::Not,
                        operand: in_node,
                    }));
                }
            }

            let inner_id = lower_expr(inner, arena, interner)?;
            let arena_op = match op {
                CoreUnaryOp::Not => ArenaUnaryOp::Not,
                CoreUnaryOp::Neg => ArenaUnaryOp::Neg,
                CoreUnaryOp::IsNull => ArenaUnaryOp::IsNull,
                CoreUnaryOp::IsNotNull => ArenaUnaryOp::IsNotNull,
                CoreUnaryOp::BitNot => ArenaUnaryOp::BitNot,
            };
            Ok(arena.alloc(ExprNode::UnaryOp {
                op: arena_op,
                operand: inner_id,
            }))
        }

        Expr::Func { name, args } => {
            let func_name = interner.intern(name.name());
            let mut arg_ids: SmallVec<[NodeId; 4]> = SmallVec::new();
            for a in args {
                arg_ids.push(lower_expr(a, arena, interner)?);
            }
            let fid = arena.alloc_func(FuncNode {
                name: func_name,
                args: arg_ids,
            });
            Ok(arena.alloc(ExprNode::Func(fid)))
        }

        Expr::Cast {
            expr: inner,
            as_type,
        } => {
            let inner_id = lower_expr(inner, arena, interner)?;
            // Use the stable `DataType::type_name()` rather than a `Debug`
            // rendering so the interned type name is wire-stable across
            // Rust toolchain upgrades.
            let type_name = interner.intern(as_type.type_name());
            Ok(arena.alloc(ExprNode::Cast {
                expr: inner_id,
                to: type_name,
            }))
        }

        Expr::Case { whens, else_expr } => {
            let mut branches: SmallVec<[(NodeId, NodeId); 4]> = SmallVec::new();
            for (c, t) in whens {
                let cid = lower_expr(c, arena, interner)?;
                let tid = lower_expr(t, arena, interner)?;
                branches.push((cid, tid));
            }
            let else_id = match else_expr.as_deref() {
                Some(e) => lower_expr(e, arena, interner)?,
                None => NULL_NODE,
            };
            let cid = arena.alloc_case(crate::CaseNode {
                branches,
                else_: else_id,
            });
            Ok(arena.alloc(ExprNode::Case(cid)))
        }

        Expr::Between {
            expr: inner,
            low,
            high,
        } => {
            let eid = lower_expr(inner, arena, interner)?;
            let lo = lower_expr(low, arena, interner)?;
            let hi = lower_expr(high, arena, interner)?;
            Ok(arena.alloc(ExprNode::Between { expr: eid, lo, hi }))
        }

        Expr::InList { expr: inner, list } => {
            let eid = lower_expr(inner, arena, interner)?;
            let mut ids: SmallVec<[NodeId; 8]> = SmallVec::new();
            for e in list {
                ids.push(lower_expr(e, arena, interner)?);
            }
            let in_id = arena.alloc_in_list(InListNode {
                expr: eid,
                list: ids,
            });
            Ok(arena.alloc(ExprNode::InList(in_id)))
        }

        Expr::Alias { expr: inner, alias } => {
            let inner_id = lower_expr(inner, arena, interner)?;
            let aid = interner.intern(alias.as_str());
            Ok(arena.alloc(ExprNode::Alias {
                expr: inner_id,
                name: aid,
            }))
        }

        Expr::Star => {
            let col = interner.intern("*");
            let fid = arena.alloc_field(FieldNode {
                namespace: None,
                name: col,
                steps: SmallVec::new(),
            });
            Ok(arena.alloc(ExprNode::Field(fid)))
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
            Ok(arena.alloc(ExprNode::Agg {
                func: func_name,
                expr: star_node,
                distinct: false,
            }))
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
                partition.push(lower_expr(e, arena, interner)?);
            }
            let mut order: SmallVec<[(NodeId, Order); 2]> = SmallVec::new();
            for ob in order_by {
                let eid = lower_expr(&ob.expr, arena, interner)?;
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
            Ok(arena.alloc(ExprNode::Window(wid)))
        }
    }
}

/// Lower an `OrderByExpr<'static>` into the arena, returning `(NodeId, Order)`.
pub fn lower_order_by(
    ob: &OrderByExpr<'_>,
    arena: &mut ExprArena,
    interner: &mut Interner,
) -> Result<(NodeId, Order), LowerError> {
    let nid = lower_expr(&ob.expr, arena, interner)?;
    let dir = match ob.direction {
        Direction::Asc => Order::Asc,
        Direction::Desc => Order::Desc,
    };
    Ok((nid, dir))
}

/// Lower multiple expressions, returning `NodeId`s.
pub fn lower_exprs(
    exprs: &[Expr<'_>],
    arena: &mut ExprArena,
    interner: &mut Interner,
) -> Result<SmallVec<[NodeId; 8]>, LowerError> {
    let mut out: SmallVec<[NodeId; 8]> = SmallVec::new();
    for e in exprs {
        out.push(lower_expr(e, arena, interner)?);
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
/// `NodeId` (or `NULL_NODE` if the list is empty).
pub fn lower_filters(
    filters: &[Expr<'_>],
    arena: &mut ExprArena,
    interner: &mut Interner,
) -> Result<NodeId, LowerError> {
    if filters.is_empty() {
        return Ok(NULL_NODE);
    }
    let mut ids: Vec<NodeId> = Vec::with_capacity(filters.len());
    for f in filters {
        ids.push(lower_expr(f, arena, interner)?);
    }
    let mut result = ids.remove(0);
    for id in ids {
        result = arena.alloc(ExprNode::BinOp {
            op: BinOp::And,
            lhs: result,
            rhs: id,
        });
    }
    Ok(result)
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
        let id = lower_expr(&field("email"), &mut arena, &mut interner).unwrap();
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
        let id = lower_expr(&expr, &mut arena, &mut interner).unwrap();
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
        let id = lower_expr(&namespace("schema.users"), &mut arena, &mut interner).unwrap();
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
        let id = lower_expr(&expr, &mut arena, &mut interner).unwrap();
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
        match arena.get(id) {
            ExprNode::UnaryOp { op, .. } => assert_eq!(*op, ArenaUnaryOp::BitNot),
            other => panic!("expected ExprNode::UnaryOp, got {:?}", other),
        }
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
        match arena.get(id) {
            ExprNode::Cast { to, .. } => {
                // `DataType::Int64` has the stable name `"int64"`.
                assert_eq!(interner.get(*to), "int64");
            }
            other => panic!("expected ExprNode::Cast, got {:?}", other),
        }
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
}
