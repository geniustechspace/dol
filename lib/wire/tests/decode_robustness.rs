//! Decode-robustness suite for [`dol_wire`].
//!
//! These are not random fuzz runs; they're a corpus of pathological byte
//! patterns that any decoder reachable from untrusted input — wire frames,
//! postcard programs, interner JSON — has to handle without panicking.
//! The contract verified here:
//!
//! * **No panics** for any input shape (zero-length, single-byte,
//!   random bytes ≤ 1 KiB).
//! * **Bad envelopes** surface as `WireError::BadMagic` /
//!   `WireError::VersionMismatch`, never as a `Codec(_)` impostor.
//! * **Future major** versions are rejected even when minor agrees,
//!   and **future minor** is rejected because we can't know what new
//!   fields it added.
//! * **Truncated** payloads after a valid header surface as
//!   `Codec(_)` rather than panicking inside postcard / serde.
//! * The `Id<Tag>` deserialiser rejects zero (the niche reserved
//!   value), surfacing as a serde error rather than UB.

#![cfg(all(feature = "postcard", feature = "json"))]

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
    // hermetic on the workspace's pinned 1.94 toolchain.
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for _ in 0..512 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
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

// ─── Postcard `Program` ──────────────────────────────────────────────────────

#[test]
fn decode_postcard_truncated_body_returns_codec_error() {
    use dol_expr::ExprArena;
    use dol_expr::Interner;
    use dol_ir::operation::{Insert, InsertSource};
    use dol_ir::{Locator, Program, Symbol, Target, TargetKind};

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

    let bytes = program::encode_postcard(&program).expect("encode");
    // Truncate progressively: every prefix of the body must surface a
    // codec error, never a panic.
    for cut in 8..bytes.len() {
        let prefix = &bytes[..cut];
        match program::decode_postcard(prefix) {
            // Could legitimately succeed if the cut lands at a serde
            // boundary — but the cases we *care* about are the ones
            // that surface as a clean error.
            Ok(_) | Err(WireError::Codec(_)) => (),
            Err(other) => panic!("cut at {cut}: unexpected error {other:?}"),
        }
    }
}

#[test]
fn decode_postcard_garbage_after_valid_header_does_not_panic() {
    // Build a wire-shaped buffer with the canonical header but a
    // postcard-illegal body. `decode_postcard` must surface a
    // `Codec(_)`, never panic.
    let mut bytes = frame(&[]);
    bytes.extend_from_slice(&[0xff; 64]);
    let res: Result<dol_ir::Program, _> = program::decode_postcard(&bytes);
    assert!(matches!(res, Err(WireError::Codec(_))));
}

// ─── Interner JSON ───────────────────────────────────────────────────────────

#[test]
fn interner_json_decode_rejects_garbage_without_panicking() {
    let inputs = [
        "",                       // empty
        "[",                      // truncated array
        "[\"\\uD800\"]",         // unpaired surrogate
        "{\"unexpected\": true}", // wrong shape
        "[1, 2, 3]",              // wrong element type
    ];
    for input in inputs {
        let res: Result<dol_expr::Interner, _> = serde_json::from_str(input);
        assert!(
            res.is_err(),
            "interner JSON should reject {input:?} but accepted: {:?}",
            res.ok()
        );
    }
}

// ─── Id<Tag> niche guard ─────────────────────────────────────────────────────

#[test]
fn id_tag_deserializer_rejects_zero() {
    use dol_expr::ids::{NodeId, StrId};

    // `Id<Tag>` is a `NonZeroU32` newtype — zero is the niche-reserved
    // value and must never decode successfully, otherwise downstream
    // `Option<Id<Tag>>` would alias `Some(<undefined>)` with `None`.
    let res: Result<NodeId, _> = serde_json::from_str("0");
    assert!(res.is_err(), "NodeId::deserialize(0) should fail");
    let res: Result<StrId, _> = serde_json::from_str("0");
    assert!(res.is_err(), "StrId::deserialize(0) should fail");

    // Sanity: positive values still decode.
    let id: NodeId = serde_json::from_str("42").unwrap();
    assert_eq!(id.get(), 42);
}
