//! Wire-format round-trip gate for `dol-wire`.
//!
//! `dol-wire` exposes typed [`dol_ir::Program`] codecs (postcard + JSON) on
//! top of the canonical wire envelope. This test file is the dedicated CI
//! gate (see `.github/workflows/ci.yml`) ensuring every supported
//! encoder/decoder pair round-trips a representative `Program`.

#![cfg(all(feature = "postcard", feature = "json"))]

use dol_expr::{ExprArena, Interner};
use dol_ir::operation::{Insert, InsertSource};
use dol_ir::{Locator, Operation, Program, Symbol, Target, TargetKind};
use dol_wire::program;

fn sample_program() -> Program {
    let mut interner = Interner::new();
    let users = Symbol::new(interner.intern("users"));
    let op: Operation = Insert {
        target: Target::new(TargetKind::Relation, Locator::new(users)),
        source: InsertSource::Bindings,
        returning: None,
    }
    .into();
    Program::new(op, ExprArena::new(), interner)
}

#[test]
fn program_postcard_round_trips() {
    let program = sample_program();
    let bytes = program::encode_postcard(&program).expect("encode_postcard");
    let decoded = program::decode_postcard(&bytes).expect("decode_postcard");
    assert_eq!(program.operations.len(), decoded.operations.len());
    assert_eq!(program.operations[0].kind(), decoded.operations[0].kind());
}

#[test]
fn program_json_round_trips() {
    let program = sample_program();
    let s = program::encode_json(&program).expect("encode_json");
    let decoded = program::decode_json(&s).expect("decode_json");
    assert_eq!(program.operations.len(), decoded.operations.len());
    assert_eq!(program.operations[0].kind(), decoded.operations[0].kind());
}

#[cfg(feature = "hash")]
#[test]
fn program_content_hash_is_stable() {
    let program = sample_program();
    let h1 = program::content_hash(&program).expect("content_hash");
    let h2 = program::content_hash(&program).expect("content_hash");
    assert_eq!(h1, h2);
}
