//! Wire-format round-trip gate for `dol-wire`.
//!
//! `dol-wire` exposes typed [`dol_ir::Program`] codecs on top of the
//! canonical wire envelope. v2 encode is `serde::Serialize`-based
//! (postcard binary, JSON for human inspection); v2 decode is
//! [`dol_wire::Decode`]-based (no `serde::Deserialize` involvement).
//! This test file is the dedicated CI gate (see
//! `.github/workflows/ci.yml`) ensuring the encode/decode pair
//! round-trips a representative `Program`.

#![cfg(all(feature = "postcard", feature = "json"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

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
    use dol_core::policy::{Budget, Limits};
    use dol_wire::encoder::encode_to_vec;

    let program = sample_program();
    // v2: encode/decode are symmetric `Encode`/`Decode` (budget-threaded).
    // Postcard-serde encode (`program::encode_postcard`) is retained for
    // content-hashing and human inspection but is **not** the matched
    // pair with `program::decode`.
    let mut budget = Budget::new(Limits::host());
    let bytes = encode_to_vec(&program, &mut budget).expect("encode");
    let decoded = program::decode(&bytes).expect("decode");
    assert_eq!(program.operations.len(), decoded.operations.len());
    assert_eq!(program.operations[0].kind(), decoded.operations[0].kind());
}

#[test]
fn program_postcard_serde_encode_is_well_formed() {
    let program = sample_program();
    // Serde-postcard encode is encode-only in v2 (used for content
    // hashing / debug). It produces an envelope-prefixed byte buffer.
    let bytes = program::encode_postcard(&program).expect("encode_postcard");
    assert!(bytes.len() > 8, "envelope header missing");
}

#[test]
fn program_json_encode_is_well_formed() {
    // JSON is encode-only in v2 (debug / language-bridge interop). The
    // round-trip contract for binary wire payloads is exercised by
    // `program_postcard_round_trips` above.
    let program = sample_program();
    let s = program::encode_json(&program).expect("encode_json");
    assert!(
        s.starts_with("{"),
        "JSON envelope must be a top-level object"
    );
    assert!(
        s.contains("\"magic\":\"DOL\""),
        "JSON envelope missing magic"
    );
}

#[cfg(feature = "hash")]
#[test]
fn program_content_hash_is_stable() {
    let program = sample_program();
    let h1 = program::content_hash(&program).expect("content_hash");
    let h2 = program::content_hash(&program).expect("content_hash");
    assert_eq!(h1, h2);
}
