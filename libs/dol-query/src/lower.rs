//! Lowering bridge: converts `dol_expr::tree::Expr<'static>` into the
//! arena-based `dol-expr` representation (`ExprArena` + `NodeId`).
//!
//! This module bridges the gap between the tree-based expression AST (with
//! lifetime-parameterised `Expr<'a>`) and the arena-based IR used by
//! `dol-ir::Statement`.

use dol_expr::tree::{Expr, OrderByExpr, Direction};
use dol_expr::arena::{ExprArena, FieldNode, FuncNode, InListNode, ObjLitNode};
use dol_expr::expr::{BinOp, ExprNode, Order, UnaryOp as ArenaUnaryOp};
use dol_expr::ids::{NodeId, NULL_NODE};
use dol_expr::interner::Interner;
use smallvec::SmallVec;

/// Lowers an `Expr<'static>` into the arena, returning the root `NodeId`.
///
/// All strings are interned into `interner`.  Sub-expressions are recursively
/// lowered in post-order.
pub fn lower_expr(
    expr: &Expr<'static>,
    arena: &mut ExprArena,
    interner: &mut Interner,
) -> NodeId {
    match expr {
        Expr::Namespace(path) => {
            let segments: Vec<&str> = path.iter().collect();
            if segments.len() == 1 {
                let col = interner.intern(segments[0]);
                let fid = arena.alloc_field(FieldNode {
                    namespace: None,
                    column: col,
                    steps: SmallVec::new(),
                });
                arena.alloc(ExprNode::Field(fid))
            } else if segments.len() == 2 {
                let ns  = interner.intern(segments[0]);
                let col = interner.intern(segments[1]);
                let fid = arena.alloc_field(FieldNode {
                    namespace: Some(ns),
                    column: col,
                    steps: SmallVec::new(),
                });
                arena.alloc(ExprNode::Field(fid))
            } else {
                let prefix = segments[..segments.len() - 1].join(".");
                let ns  = interner.intern(&prefix);
                let col = interner.intern(segments[segments.len() - 1]);
                let fid = arena.alloc_field(FieldNode {
                    namespace: Some(ns),
                    column: col,
                    steps: SmallVec::new(),
                });
                arena.alloc(ExprNode::Field(fid))
            }
        }

        Expr::Field { base, path } => {
            let base_id = lower_expr(base, arena, interner);
            let segments: Vec<&str> = path.iter().collect();
            let mut result = base_id;
            for seg in segments {
                let key_id = interner.intern(seg);
                let key_node = arena.alloc(ExprNode::Namespace(key_id));
                result = arena.alloc(ExprNode::BinOp {
                    op: BinOp::Arrow,
                    lhs: result,
                    rhs: key_node,
                });
            }
            result
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
            arena.alloc(ExprNode::BinOp { op: bin_op, lhs, rhs })
        }

        Expr::UnaryOp { op, expr: inner } => {
            use dol_expr::tree::UnaryOp as CoreUnaryOp;

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
            arena.alloc(ExprNode::UnaryOp { op: arena_op, operand: inner_id })
        }

        Expr::Func { name, args } => {
            let func_name = interner.intern(name.name());
            let arg_ids: SmallVec<[NodeId; 4]> = args
                .iter()
                .map(|a| lower_expr(a, arena, interner))
                .collect();
            let fid = arena.alloc_func(FuncNode { name: func_name, args: arg_ids });
            arena.alloc(ExprNode::Func(fid))
        }

        Expr::Cast { expr: inner, as_type } => {
            let inner_id = lower_expr(inner, arena, interner);
            let type_name = interner.intern(&format!("{:?}", as_type));
            arena.alloc(ExprNode::Cast { expr: inner_id, to: type_name })
        }

        Expr::Case { whens, else_expr } => {
            let branches: SmallVec<[(NodeId, NodeId); 4]> = whens
                .iter()
                .map(|(c, t)| {
                    (lower_expr(c, arena, interner), lower_expr(t, arena, interner))
                })
                .collect();
            let else_id = else_expr
                .as_deref()
                .map(|e| lower_expr(e, arena, interner))
                .unwrap_or(NULL_NODE);
            let cid = arena.alloc_case(dol_expr::CaseNode { branches, else_: else_id });
            arena.alloc(ExprNode::Case(cid))
        }

        Expr::Between { expr: inner, low, high } => {
            let eid = lower_expr(inner, arena, interner);
            let lo  = lower_expr(low, arena, interner);
            let hi  = lower_expr(high, arena, interner);
            arena.alloc(ExprNode::Between { expr: eid, lo, hi })
        }

        Expr::InList { expr: inner, list } => {
            let eid = lower_expr(inner, arena, interner);
            let ids: SmallVec<[NodeId; 8]> = list
                .iter()
                .map(|e| lower_expr(e, arena, interner))
                .collect();
            let in_id = arena.alloc_in_list(InListNode { expr: eid, list: ids });
            arena.alloc(ExprNode::InList(in_id))
        }

        Expr::Alias { expr: inner, alias } => {
            let inner_id = lower_expr(inner, arena, interner);
            let aid = interner.intern(alias.as_str());
            arena.alloc(ExprNode::Alias { expr: inner_id, name: aid })
        }

        Expr::Star => {
            let col = interner.intern("*");
            let fid = arena.alloc_field(FieldNode {
                namespace: None,
                column: col,
                steps: SmallVec::new(),
            });
            arena.alloc(ExprNode::Field(fid))
        }

        Expr::CountStar => {
            let func_name = interner.intern("count");
            let star_col = interner.intern("*");
            let star_fid = arena.alloc_field(FieldNode {
                namespace: None,
                column: star_col,
                steps: SmallVec::new(),
            });
            let star_node = arena.alloc(ExprNode::Field(star_fid));
            arena.alloc(ExprNode::Agg { func: func_name, expr: star_node, distinct: false })
        }

        Expr::Window { func, partition_by, order_by, .. } => {
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
            let wid = arena.alloc_window(dol_expr::WindowNode {
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
    ob: &OrderByExpr<'static>,
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
    exprs: &[Expr<'static>],
    arena: &mut ExprArena,
    interner: &mut Interner,
) -> SmallVec<[NodeId; 8]> {
    exprs.iter().map(|e| lower_expr(e, arena, interner)).collect()
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
    filters: &[Expr<'static>],
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
        result = arena.alloc(ExprNode::BinOp { op: BinOp::And, lhs: result, rhs: id });
    }
    result
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Map `dol_expr::tree::OpDef` name to `dol-expr::expr::BinOp`.
fn lower_binop(op: &dol_expr::tree::OpDef) -> BinOp {
    match op.name() {
        "EQ"  => BinOp::Eq,
        "NE"  => BinOp::Ne,
        "LT"  => BinOp::Lt,
        "GT"  => BinOp::Gt,
        "LE"  => BinOp::Le,
        "GE"  => BinOp::Ge,
        "ADD" => BinOp::Add,
        "SUB" => BinOp::Sub,
        "MUL" => BinOp::Mul,
        "DIV" => BinOp::Div,
        "MOD" => BinOp::Rem,
        "AND" => BinOp::And,
        "OR"  => BinOp::Or,
        "LIKE"       => BinOp::Like,
        "ILIKE"      => BinOp::ILike,
        "SIMILAR_TO" => BinOp::Similar,
        "CONCAT"     => BinOp::Concat,
        "BIT_AND"      => BinOp::BitAnd,
        "BIT_OR"       => BinOp::BitOr,
        "BIT_XOR"      => BinOp::BitXor,
        "SHIFT_LEFT"   => BinOp::Shl,
        "SHIFT_RIGHT"  => BinOp::Shr,
        _ => BinOp::Eq, // Fallback for unknown operators.
    }
}
