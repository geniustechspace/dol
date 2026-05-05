//! Round-trip serde tests for `dol-expr` public top-level types.
//!
//! Heavy node types ([`QueryNode`], [`InsertNode`], …) and the [`ExprArena`]
//! itself are already exercised end-to-end via the program round-trip in
//! `dol-ir`; here we cover the small operator / enum / payload types so a
//! regression on any one of them is caught directly.

#![cfg(feature = "serde")]

use dol_expr::arena::{FieldNode, FieldStep};
use dol_expr::ids::{LiteralId, NodeId, StrId};
use dol_expr::{
    BinOp, ConflictClause, DeleteNode, ExprArena, ExprNode, Interner, JoinNode, JoinType, LockHint,
    Order, QueryNode, UnaryOp, UpdateNode,
};
use smallvec::smallvec;

fn round_trip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let json = serde_json::to_string(value).expect("serialize");
    serde_json::from_str(&json).expect("deserialize")
}

/// Helper: typed [`NodeId`] from a 1-based raw index.
fn nid(raw: u32) -> NodeId {
    NodeId::from_u32(raw).expect("non-zero")
}
/// Helper: typed [`StrId`] from a 1-based raw index.
fn sid(raw: u32) -> StrId {
    StrId::from_u32(raw).expect("non-zero")
}
/// Helper: typed [`LiteralId`] from a 1-based raw index.
fn lid(raw: u32) -> LiteralId {
    LiteralId::from_u32(raw).expect("non-zero")
}

#[test]
fn binop_round_trip() {
    for op in [
        BinOp::Eq,
        BinOp::Ne,
        BinOp::Lt,
        BinOp::Le,
        BinOp::Gt,
        BinOp::Ge,
        BinOp::And,
        BinOp::Or,
        BinOp::Add,
        BinOp::Sub,
        BinOp::Mul,
        BinOp::Div,
        BinOp::Rem,
        BinOp::Like,
        BinOp::ILike,
        BinOp::Similar,
        BinOp::BitAnd,
        BinOp::BitOr,
        BinOp::BitXor,
        BinOp::Shl,
        BinOp::Shr,
        BinOp::Concat,
        BinOp::Arrow,
        BinOp::LongArrow,
    ] {
        assert_eq!(op, round_trip(&op));
    }
}

#[test]
fn unaryop_round_trip() {
    for op in [
        UnaryOp::Neg,
        UnaryOp::Not,
        UnaryOp::IsNull,
        UnaryOp::IsNotNull,
        UnaryOp::IsTrue,
        UnaryOp::IsFalse,
    ] {
        assert_eq!(op, round_trip(&op));
    }
}

#[test]
fn order_and_lock_round_trip() {
    for o in [Order::Asc, Order::Desc] {
        assert_eq!(o, round_trip(&o));
    }
    for l in [
        LockHint::ForUpdate,
        LockHint::ForShare,
        LockHint::SkipLocked,
        LockHint::NoWait,
    ] {
        assert_eq!(l, round_trip(&l));
    }
}

#[test]
fn join_round_trip() {
    for jt in [
        JoinType::Inner,
        JoinType::Left,
        JoinType::Right,
        JoinType::Full,
        JoinType::Cross,
    ] {
        assert_eq!(jt, round_trip(&jt));
    }
    let j = JoinNode {
        source: sid(1),
        alias: Some(sid(2)),
        join_type: JoinType::Left,
        on: Some(nid(3)),
    };
    assert_eq!(j, round_trip(&j));
}

#[test]
fn conflict_clause_round_trip() {
    let cs = [
        ConflictClause::DoNothing,
        ConflictClause::DoUpdate {
            assignments: smallvec![(sid(1), nid(1)), (sid(2), nid(3))],
        },
    ];
    for c in &cs {
        assert_eq!(c, &round_trip(c));
    }
}

#[test]
fn field_step_and_node_round_trip() {
    let steps = [FieldStep::Key(sid(7)), FieldStep::Index(0)];
    for s in &steps {
        assert_eq!(s, &round_trip(s));
    }
    let fnode = FieldNode {
        namespace: Some(sid(1)),
        name: sid(2),
        steps: smallvec![FieldStep::Key(sid(3)), FieldStep::Index(0)],
    };
    assert_eq!(fnode, round_trip(&fnode));
}

#[test]
fn expr_node_simple_variants_round_trip() {
    let nodes = vec![
        ExprNode::Namespace(sid(1)),
        ExprNode::Param,
        ExprNode::Lit(lid(1)),
        ExprNode::Field(dol_expr::ids::FieldId::from_u32(1).unwrap()),
        ExprNode::ArrayLit(smallvec![nid(1), nid(2), nid(3)]),
        ExprNode::BinOp {
            op: BinOp::Eq,
            lhs: nid(1),
            rhs: nid(2),
        },
        ExprNode::UnaryOp {
            op: UnaryOp::Not,
            operand: nid(1),
        },
        ExprNode::Cast {
            expr: nid(1),
            to: sid(2),
        },
        ExprNode::Alias {
            expr: nid(1),
            name: sid(2),
        },
        ExprNode::Between {
            expr: nid(1),
            lo: nid(2),
            hi: nid(3),
        },
        ExprNode::Agg {
            func: sid(1),
            expr: nid(2),
            distinct: true,
        },
    ];
    for n in &nodes {
        assert_eq!(n, &round_trip(n));
    }
}

#[test]
fn statement_node_payloads_round_trip() {
    let q = QueryNode {
        from: sid(1),
        alias: None,
        joins: smallvec![],
        // `None` is the new "no filter" — replaces the previous
        // `u32::MAX` sentinel now that `filter: Option<NodeId>`.
        filter: None,
        columns: smallvec![nid(1), nid(2)],
        group_by: smallvec![],
        having: None,
        order_by: smallvec![(nid(1), Order::Asc)],
        limit: Some(10),
        offset: None,
        lock: Some(LockHint::ForUpdate),
    };
    assert_eq!(q, round_trip(&q));

    let upd = UpdateNode {
        target: sid(1),
        columns: smallvec![sid(1), sid(2)],
        values: smallvec![nid(3), nid(4)],
        filter: Some(nid(5)),
        returning: smallvec![nid(6)],
    };
    assert_eq!(upd, round_trip(&upd));

    let del = DeleteNode {
        target: sid(1),
        filter: Some(nid(1)),
        returning: smallvec![nid(2)],
    };
    assert_eq!(del, round_trip(&del));
}

#[test]
fn interner_and_arena_round_trip() {
    let mut interner = Interner::new();
    let _ = interner.intern("users");
    let id = interner.intern("email");

    let mut arena = ExprArena::new();
    let _n = arena.alloc(ExprNode::Namespace(id));

    let interner_back: Interner = round_trip(&interner);
    assert_eq!(interner.len(), interner_back.len());
    assert_eq!(interner.get(id), interner_back.get(id));

    let arena_back: ExprArena = round_trip(&arena);
    // Arena fields are private; structural equivalence is asserted via a
    // re-encode comparison on the JSON body, which is the canonical form.
    assert_eq!(
        serde_json::to_value(&arena).unwrap(),
        serde_json::to_value(&arena_back).unwrap(),
    );
}
