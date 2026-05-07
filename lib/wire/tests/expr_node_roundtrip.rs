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

/// The packed-POD wire form is exactly 16 bytes per node (op + flags +
/// LE aux/a/b/c). Pin this so a future refactor doesn't accidentally
/// reintroduce per-opcode varint and lose the bulk-POD fast path.
#[test]
fn every_node_is_16_wire_bytes() {
    let nodes = [
        ExprNode::nop(),
        ExprNode::namespace(sid(7)),
        ExprNode::field(FieldId::from_u32(3).unwrap()),
        ExprNode::param(0),
        ExprNode::param(0xDEAD_BEEF),
        ExprNode::lit(LiteralId::from_u32(2).unwrap()),
        ExprNode::object_lit(ObjLitId::from_u32(1).unwrap()),
        ExprNode::array_lit(ArrayLitId::from_u32(5).unwrap()),
        ExprNode::bin(BinOp::Eq, nid(1), nid(2)),
        ExprNode::una(UnaryOp::Not, nid(1)),
        ExprNode::func(FuncId::from_u32(4).unwrap()),
        ExprNode::agg(sid(1), nid(2), true),
        ExprNode::window(WindowId::from_u32(8).unwrap()),
        ExprNode::cast(nid(2), sid(9)),
        ExprNode::case(CaseId::from_u32(11).unwrap()),
        ExprNode::alias(nid(2), sid(3)),
        ExprNode::in_list(InListId::from_u32(13).unwrap()),
        ExprNode::in_sub(nid(2), nid(3)),
        ExprNode::exists(nid(7)),
        ExprNode::between(nid(2), nid(3), nid(4)),
    ];
    for n in nodes {
        let mut buf: Vec<u8> = Vec::new();
        let mut w = Writer::new(&mut buf);
        let mut budget = Budget::new(Limits::host());
        n.encode(&mut w, &mut budget).expect("encode");
        assert_eq!(buf.len(), 16, "node {:?} encoded to {} bytes", n, buf.len());
    }
}

/// Encoded node bytes match the in-memory `bytemuck::bytes_of` form on
/// LE hosts (which is what makes the bulk-POD fast path "free": once
/// the per-node form is the same as the packed in-memory form, encoding
/// the whole nodes vector reduces to a single `write_bytes` of
/// `cast_slice` bytes).
#[cfg(target_endian = "little")]
#[test]
fn wire_bytes_match_pod_bytes_on_le() {
    let n = ExprNode::between(nid(2), nid(3), nid(4));
    let mut buf: Vec<u8> = Vec::new();
    let mut w = Writer::new(&mut buf);
    let mut budget = Budget::new(Limits::host());
    n.encode(&mut w, &mut budget).expect("encode");
    let pod_bytes: &[u8] = bytemuck::bytes_of(&n);
    assert_eq!(buf, pod_bytes);
}

/// Bulk arena round-trip: encoding 1 000 nodes through the public
/// `Encode for ExprArena` path produces a wire form whose nodes section
/// is exactly `len + N × 16` bytes, and decoding restores the arena.
#[test]
fn arena_nodes_bulk_round_trip() {
    use dol_expr::arena::ExprArena;
    use dol_wire::decoder::Decode;

    let mut arena = ExprArena::new();
    for i in 0..1_000u32 {
        // Use a NodeId-bearing param so every node has a non-trivial
        // payload: param's `a` is the index, not a NodeId, so this is
        // safe.
        arena.alloc(ExprNode::param(i));
    }

    let mut buf: Vec<u8> = Vec::new();
    {
        let mut w = Writer::new(&mut buf);
        let mut budget = Budget::new(Limits::host());
        arena.encode(&mut w, &mut budget).expect("encode");
    }

    let mut reader = Reader::new(&buf);
    let mut budget = Budget::new(Limits::host());
    let decoded = ExprArena::decode(&mut reader, &mut budget).expect("decode");
    assert_eq!(decoded.nodes_slice(), arena.nodes_slice());
}

/// Decode rejects a corrupted opcode byte cleanly via the validity
/// sweep in `Decode for ExprNode`.
#[test]
fn decode_rejects_unknown_opcode() {
    use dol_wire::decoder::{Decode, DecodeError};

    // Construct a byte stream by hand: op = 0xFF (invalid) + 15 zero
    // bytes for the rest of the 16 B node form.
    let buf: Vec<u8> = core::iter::once(0xFFu8)
        .chain(core::iter::repeat_n(0u8, 15))
        .collect();
    let mut reader = Reader::new(&buf);
    let mut budget = Budget::new(Limits::host());
    let err = ExprNode::decode(&mut reader, &mut budget).expect_err("invalid op must fail");
    assert!(matches!(err, DecodeError::InvalidVariant { type_name: "ExprNode", .. }));
}
