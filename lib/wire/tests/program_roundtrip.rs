//! Round-trip tests for the typed `dol_wire::program` codecs.

#![cfg(all(feature = "postcard", feature = "json", feature = "hash"))]
#![allow(deprecated)] // exercises the `Statement`/`stmt` v1 surface intentionally

use dol_expr::expr::QueryNode;
use dol_expr::{ExprArena, ExprNode, Interner};
use dol_ir::{Program, Statement};
use smallvec::smallvec;

fn sample_program() -> Program {
    let mut interner = Interner::new();
    let mut arena = ExprArena::new();

    let from = interner.intern("users");
    let star = interner.intern("*");
    let star_node = arena.alloc(ExprNode::Namespace(star));

    let qnode = QueryNode {
        from,
        alias: None,
        joins: Default::default(),
        filter: 0,
        columns: smallvec![star_node],
        group_by: Default::default(),
        having: 0,
        order_by: Default::default(),
        limit: Some(50),
        offset: None,
        lock: None,
    };

    Program::new(Statement::Query(Box::new(qnode)), arena, interner)
}

#[test]
fn postcard_round_trip() {
    let prog = sample_program();
    let bytes = dol_wire::program::encode_postcard(&prog).expect("encode");
    let decoded = dol_wire::program::decode_postcard(&bytes).expect("decode");
    assert_eq!(prog.stmt, decoded.stmt);
    assert_eq!(prog.interner.len(), decoded.interner.len());
}

#[test]
fn json_round_trip() {
    let prog = sample_program();
    let s = dol_wire::program::encode_json(&prog).expect("encode");
    let decoded = dol_wire::program::decode_json(&s).expect("decode");
    assert_eq!(prog.stmt, decoded.stmt);
    assert_eq!(prog.interner.len(), decoded.interner.len());
}

#[test]
fn content_hash_is_stable() {
    let prog = sample_program();
    let a = dol_wire::program::content_hash(&prog).expect("hash");
    let b = dol_wire::program::content_hash(&prog).expect("hash");
    assert_eq!(a, b);
    assert_eq!(a.len(), 32);
}

#[test]
fn header_is_versioned() {
    let prog = sample_program();
    let bytes = dol_wire::program::encode_postcard(&prog).expect("encode");
    // First 8 bytes are the WireHeader: magic + version
    assert!(bytes.len() > 8);
    let header_bytes: [u8; 8] = bytes[..8].try_into().unwrap();
    let header = dol_wire::WireHeader::from_bytes(header_bytes).expect("valid header");
    assert_eq!(header.version, dol_wire::CURRENT_VERSION);
}
