//! Round-trip serde tests for `dol-expr` public top-level types.
//!
//! Heavy node types ([`QueryNode`], [`InsertNode`], …) and the [`ExprArena`]
//! itself are already exercised end-to-end via the program round-trip in
//! `dol-ir`; here we cover the small operator / enum / payload types so a
//! regression on any one of them is caught directly.

#![cfg(feature = "serde")]

use dol_expr::arena::{FieldNode, FieldStep};
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
        source: 1,
        alias: Some(2),
        join_type: JoinType::Left,
        on: 3,
    };
    assert_eq!(j, round_trip(&j));
}

#[test]
fn conflict_clause_round_trip() {
    let cs = [
        ConflictClause::DoNothing,
        ConflictClause::DoUpdate {
            assignments: smallvec![(0u32, 1u32), (2, 3)],
        },
    ];
    for c in &cs {
        assert_eq!(c, &round_trip(c));
    }
}

#[test]
fn field_step_and_node_round_trip() {
    let steps = [FieldStep::Key(7), FieldStep::Index(0)];
    for s in &steps {
        assert_eq!(s, &round_trip(s));
    }
    let fnode = FieldNode {
        namespace: Some(1),
        name: 2,
        steps: smallvec![FieldStep::Key(3), FieldStep::Index(0)],
    };
    assert_eq!(fnode, round_trip(&fnode));
}

#[test]
fn expr_node_simple_variants_round_trip() {
    let nodes = vec![
        ExprNode::Namespace(0),
        ExprNode::Param,
        ExprNode::Lit(0),
        ExprNode::Field(0),
        ExprNode::ArrayLit(smallvec![1, 2, 3]),
        ExprNode::BinOp {
            op: BinOp::Eq,
            lhs: 1,
            rhs: 2,
        },
        ExprNode::UnaryOp {
            op: UnaryOp::Not,
            operand: 1,
        },
        ExprNode::Cast { expr: 1, to: 2 },
        ExprNode::Alias { expr: 1, name: 2 },
        ExprNode::Between {
            expr: 1,
            lo: 2,
            hi: 3,
        },
        ExprNode::Agg {
            func: 1,
            expr: 2,
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
        from: 0,
        alias: None,
        joins: smallvec![],
        filter: u32::MAX,
        columns: smallvec![1, 2],
        group_by: smallvec![],
        having: u32::MAX,
        order_by: smallvec![(1, Order::Asc)],
        limit: Some(10),
        offset: None,
        lock: Some(LockHint::ForUpdate),
    };
    assert_eq!(q, round_trip(&q));

    let upd = UpdateNode {
        target: 0,
        columns: smallvec![1, 2],
        values: smallvec![3, 4],
        filter: 5,
        returning: smallvec![6],
    };
    assert_eq!(upd, round_trip(&upd));

    let del = DeleteNode {
        target: 0,
        filter: 1,
        returning: smallvec![2],
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
