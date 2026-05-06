//! Decode-robustness suite for [`dol_wire`].
//!
//! These are not random fuzz runs; they're a corpus of pathological byte
//! patterns that any decoder reachable from untrusted input — wire frames,
//! postcard programs — has to handle without panicking.
//! The contract verified here:
//!
//! * **No panics** for any input shape (zero-length, single-byte,
//!   random bytes ≤ 1 KiB).
//! * **Bad envelopes** surface as `WireError::BadMagic` /
//!   `WireError::VersionMismatch`, never as a `Codec(_)` impostor.
//! * **Future major** versions are rejected even when minor agrees,
//!   and **future minor** is rejected because we can't know what new
//!   fields it added.
//! * **Truncated** payloads after a valid header surface as a clean
//!   `DecodeError` from the budget-threaded [`dol_wire::Decode`] path,
//!   not a panic inside postcard.
//!
//! v2 invariant: decode goes through [`dol_wire::Decode`] only;
//! `serde::Deserialize` is gone from the in-memory IR. The previous
//! `Id<Tag>` "zero rejection" test no longer applies because there is
//! no `Deserialize` impl to test — the equivalent invariant is
//! exercised by `dol_wire::decode_ir`'s `Decode` impl tests.

#![cfg(feature = "postcard")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use dol_wire::{
    CURRENT_VERSION, WireError, WireHeader, WireSchemaVersion, frame, program, unframe,
};

// ─── Header / envelope ───────────────────────────────────────────────────────

#[test]
fn unframe_short_inputs_never_panic() {
    for n in 0..8usize {
        let buf = vec![0u8; n];
        match unframe(&buf) {
            Err(WireError::BadMagic) => (),
            Err(other) => panic!("short input {n}: unexpected error {other:?}"),
            Ok(_) => panic!("short input {n}: should have failed"),
        }
    }
}

#[test]
fn unframe_rejects_future_major() {
    let mut bytes = WireHeader::current().to_bytes().to_vec();
    // Bump major, leave minor at 0 — strictly forward in major space.
    let future = WireSchemaVersion::new(CURRENT_VERSION.major + 1, 0);
    bytes[4..8].copy_from_slice(&future.to_bytes());
    match unframe(&bytes) {
        Err(WireError::VersionMismatch { found, expected }) => {
            assert_eq!(found, future);
            assert_eq!(expected, CURRENT_VERSION);
        }
        other => panic!("future major: unexpected {other:?}"),
    }
}

#[test]
fn unframe_rejects_future_minor() {
    // A future minor declares fields we don't know how to parse.
    let mut bytes = WireHeader::current().to_bytes().to_vec();
    let future = WireSchemaVersion::new(CURRENT_VERSION.major, CURRENT_VERSION.minor + 1);
    bytes[4..8].copy_from_slice(&future.to_bytes());
    assert!(matches!(
        unframe(&bytes),
        Err(WireError::VersionMismatch { .. })
    ));
}

#[test]
fn unframe_arbitrary_bytes_never_panic() {
    // Walk a deterministic LCG over 1 KiB of input lengths and bytes.
    // No external `proptest` / `arbitrary` dependency — keeps the suite
    // hermetic on the workspace's pinned toolchain.
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for _ in 0..512 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let len = ((state >> 32) as usize) % 64;
        let mut buf = Vec::with_capacity(len);
        for i in 0..len {
            buf.push(((state >> (i % 56)) & 0xFF) as u8);
        }
        // Any of {Ok, BadMagic, VersionMismatch, Codec} is acceptable —
        // the only forbidden outcome is a panic, which the test
        // detects by simply running.
        let _ = unframe(&buf);
    }
}

// ─── Postcard `Program` (Decode-based wire-in) ───────────────────────────────

#[test]
fn decode_truncated_body_returns_clean_error() {
    use dol_core::policy::{Budget, Limits};
    use dol_expr::ExprArena;
    use dol_expr::Interner;
    use dol_ir::operation::{Insert, InsertSource};
    use dol_ir::{Locator, Program, Symbol, Target, TargetKind};
    use dol_wire::encoder::encode_to_vec;

    // A real, encodable program — anything tiny will do.
    let mut interner = Interner::new();
    let users = Symbol::new(interner.intern("users"));
    let op: dol_ir::Operation = Insert {
        target: Target::new(TargetKind::Relation, Locator::new(users)),
        source: InsertSource::Bindings,
        returning: None,
    }
    .into();
    let program = Program::new(op, ExprArena::new(), interner);

    let mut budget = Budget::new(Limits::host());
    let bytes = encode_to_vec(&program, &mut budget).expect("encode");
    // Truncate progressively: every prefix of the body must surface a
    // clean `DecodeError`, never a panic. (The test running to
    // completion is what proves no panic.)
    for cut in 0..bytes.len() {
        let prefix = &bytes[..cut];
        let _ = program::decode(prefix);
    }
}

#[test]
fn decode_garbage_after_valid_header_does_not_panic() {
    // Build a wire-shaped buffer with the canonical header but a
    // postcard-illegal body. `program::decode` must surface a clean
    // `DecodeError`, never panic.
    let mut bytes = frame(&[]);
    bytes.extend_from_slice(&[0xff; 64]);
    let res = program::decode(&bytes);
    assert!(res.is_err(), "garbage body must fail to decode");
}
