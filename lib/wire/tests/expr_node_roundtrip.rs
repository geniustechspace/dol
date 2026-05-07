//! Per-opcode wire-format round-trip gate for `dol-expr::ExprNode`.
//!
//! The v2 packed `ExprNode` encodes opcode-by-opcode (see
//! `lib/wire/src/encode_expr.rs`); this test pins the contract by
//! constructing one node of each opcode, encoding it through
//! `dol-wire::Encode`, decoding it back through `dol-wire::Decode`,
//! and asserting the byte-for-byte identity of the result.
//!
//! If a future opcode is added or the layout is changed, the
//! corresponding case here must be extended — that is the point.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use dol_core::policy::{Budget, Limits};
use dol_expr::expr::{BinOp, ExprNode, UnaryOp};
use dol_expr::ids::{
    ArrayLitId, CaseId, DeleteId, FieldId, FuncId, InListId, InsertId, LiteralId, NodeId,
    ObjLitId, QueryId, StrId, UpdateId, UpsertId, WindowId,
};
use dol_wire::decoder::{Decode, Reader};
use dol_wire::encoder::{Encode, Writer};

fn round_trip(node: ExprNode) -> ExprNode {
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut w = Writer::new(&mut buf);
        let mut budget = Budget::new(Limits::host());
        node.encode(&mut w, &mut budget).expect("encode");
    }
    let mut reader = Reader::new(&buf);
    let mut budget = Budget::new(Limits::host());
    let decoded = ExprNode::decode(&mut reader, &mut budget).expect("decode");
    assert_eq!(reader.remaining(), 0, "decoder did not consume full buffer");
    decoded
}

fn nid(n: u32) -> NodeId {
    NodeId::from_u32(n).expect("non-zero")
}
fn sid(n: u32) -> StrId {
    StrId::from_u32(n).expect("non-zero")
}

#[test]
fn nop_round_trips() {
    let n = ExprNode::nop();
    assert_eq!(round_trip(n), n);
}

#[test]
fn namespace_round_trips() {
    let n = ExprNode::namespace(sid(7));
    assert_eq!(round_trip(n), n);
}

#[test]
fn field_round_trips() {
    let n = ExprNode::field(FieldId::from_u32(3).unwrap());
    assert_eq!(round_trip(n), n);
}

#[test]
fn param_round_trips() {
    let n = ExprNode::param(0);
    assert_eq!(round_trip(n), n);
    let n = ExprNode::param(42);
    assert_eq!(round_trip(n), n);
}

#[test]
fn lit_round_trips() {
    let n = ExprNode::lit(LiteralId::from_u32(2).unwrap());
    assert_eq!(round_trip(n), n);
}

#[test]
fn object_lit_round_trips() {
    let n = ExprNode::object_lit(ObjLitId::from_u32(1).unwrap());
    assert_eq!(round_trip(n), n);
}

#[test]
fn array_lit_round_trips() {
    let n = ExprNode::array_lit(ArrayLitId::from_u32(5).unwrap());
    assert_eq!(round_trip(n), n);
}

#[test]
fn bin_round_trips_for_every_op() {
    for tag in 0u16..=23 {
        let op = BinOp::try_from_u16(tag).expect("known tag");
        let n = ExprNode::bin(op, nid(2), nid(3));
        assert_eq!(round_trip(n), n, "op tag {tag}");
    }
}

#[test]
fn una_round_trips_for_every_op() {
    for tag in 0u16..=6 {
        let op = UnaryOp::try_from_u16(tag).expect("known tag");
        let n = ExprNode::una(op, nid(2));
        assert_eq!(round_trip(n), n, "op tag {tag}");
    }
}

#[test]
fn func_round_trips() {
    let n = ExprNode::func(FuncId::from_u32(4).unwrap());
    assert_eq!(round_trip(n), n);
}

#[test]
fn agg_round_trips_with_distinct_flag() {
    let n = ExprNode::agg(sid(1), nid(2), false);
    assert_eq!(round_trip(n), n);
    let n = ExprNode::agg(sid(1), nid(2), true);
    assert_eq!(round_trip(n), n);
}

#[test]
fn window_round_trips() {
    let n = ExprNode::window(WindowId::from_u32(8).unwrap());
    assert_eq!(round_trip(n), n);
}

#[test]
fn cast_round_trips() {
    let n = ExprNode::cast(nid(2), sid(9));
    assert_eq!(round_trip(n), n);
}

#[test]
fn case_round_trips() {
    let n = ExprNode::case(CaseId::from_u32(11).unwrap());
    assert_eq!(round_trip(n), n);
}

#[test]
fn alias_round_trips() {
    let n = ExprNode::alias(nid(2), sid(3));
    assert_eq!(round_trip(n), n);
}

#[test]
fn in_list_round_trips() {
    let n = ExprNode::in_list(InListId::from_u32(13).unwrap());
    assert_eq!(round_trip(n), n);
}

#[test]
fn in_sub_round_trips() {
    let n = ExprNode::in_sub(nid(2), nid(3));
    assert_eq!(round_trip(n), n);
}

#[test]
fn exists_round_trips() {
    let n = ExprNode::exists(nid(7));
    assert_eq!(round_trip(n), n);
}

#[test]
fn between_round_trips() {
    let n = ExprNode::between(nid(2), nid(3), nid(4));
    assert_eq!(round_trip(n), n);
}

#[test]
fn dml_node_refs_round_trip() {
    assert_eq!(
        round_trip(ExprNode::query(QueryId::from_u32(1).unwrap())),
        ExprNode::query(QueryId::from_u32(1).unwrap())
    );
    assert_eq!(
        round_trip(ExprNode::insert(InsertId::from_u32(2).unwrap())),
        ExprNode::insert(InsertId::from_u32(2).unwrap())
    );
    assert_eq!(
        round_trip(ExprNode::update(UpdateId::from_u32(3).unwrap())),
        ExprNode::update(UpdateId::from_u32(3).unwrap())
    );
    assert_eq!(
        round_trip(ExprNode::delete(DeleteId::from_u32(4).unwrap())),
        ExprNode::delete(DeleteId::from_u32(4).unwrap())
    );
    assert_eq!(
        round_trip(ExprNode::upsert(UpsertId::from_u32(5).unwrap())),
        ExprNode::upsert(UpsertId::from_u32(5).unwrap())
    );
}
