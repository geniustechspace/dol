//! Typed [`dol_ir::program::Program`] codecs.
//!
//! These helpers wrap the underlying postcard helper in [`crate::postcard`]
//! and the validating-decode helper [`crate::decoder::Decode`] for
//! [`dol_ir::program::Program`] directly.
//!
//! v2 invariant: encode goes through `serde::Serialize` (postcard / JSON),
//! but **decode goes through [`crate::Decode`] only**. Generic
//! `decode_postcard<T: Deserialize>` / `decode_json<T: Deserialize>`
//! helpers were removed in 0.2.0; callers that need to wire-in a
//! `Program` use [`decode`] below, which threads `&mut Budget` end-to-end.

#[cfg(feature = "json")]
use alloc::string::String;
use alloc::vec::Vec;

use dol_ir::program::Program;

use crate::WireError;

/// Postcard-encode a [`Program`] with the canonical [`crate::WireHeader`].
#[cfg(feature = "postcard")]
pub fn encode_postcard(program: &Program) -> Result<Vec<u8>, WireError> {
    crate::postcard::encode(program)
}

/// JSON-encode a [`Program`] with the canonical wire envelope (string form).
#[cfg(feature = "json")]
pub fn encode_json(program: &Program) -> Result<String, WireError> {
    crate::json::encode(program)
}

/// BLAKE3 content hash over the canonical postcard body of a [`Program`].
///
/// This is the recommended identity for migration keys, plan-cache keys,
/// and IoT idempotency tokens.
#[cfg(all(feature = "postcard", feature = "hash"))]
pub fn content_hash(program: &Program) -> Result<crate::hash::Digest, WireError> {
    crate::hash::content_hash(program)
}

/// Decode a [`Program`] from raw postcard bytes using the budget-aware
/// [`crate::decoder::Decode`] path (no serde dependency).
// budget-gate: opt-out: this is the public entry point that *constructs*
// the `Budget`; downstream calls to `Program::decode` thread it. The
// gate's pattern matcher only sees the surface signature, hence the
// opt-out.
#[cfg(feature = "postcard")]
pub fn decode(bytes: &[u8]) -> Result<Program, crate::decoder::DecodeError> {
    use crate::decoder::Decode;
    use dol_core::policy::Limits;
    let mut reader = crate::decoder::Reader::new(bytes);
    let mut budget = dol_core::policy::Budget::new(Limits::default());
    Program::decode(&mut reader, &mut budget)
}
