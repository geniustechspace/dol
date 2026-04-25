//! Typed [`dol_ir::Program`] codecs.
//!
//! These helpers wrap the underlying postcard / JSON helpers in
//! [`crate::postcard`] and [`crate::json`] and target [`dol_ir::Program`]
//! directly, so callers don't need to keep the body type parameter in sync
//! with the rest of the IR.
//!
//! All of these helpers re-use the canonical [`crate::WireHeader`], so
//! payloads emitted here are byte-compatible with anything decoded by
//! [`crate::postcard::decode`] or [`crate::json::decode`] when the body
//! type is `dol_ir::Program`.
//!
//! Both the postcard and JSON helpers are gated behind the matching feature
//! flag (`postcard`, `json`) and the universal `serde` plumbing.

#[cfg(feature = "json")]
use alloc::string::String;
use alloc::vec::Vec;

use dol_ir::Program;

use crate::WireError;

/// Postcard-encode a [`Program`] with the canonical [`crate::WireHeader`].
#[cfg(feature = "postcard")]
pub fn encode_postcard(program: &Program) -> Result<Vec<u8>, WireError> {
    crate::postcard::encode(program)
}

/// Decode a postcard-encoded [`Program`].
#[cfg(feature = "postcard")]
pub fn decode_postcard(bytes: &[u8]) -> Result<Program, WireError> {
    crate::postcard::decode(bytes)
}

/// JSON-encode a [`Program`] with the canonical wire envelope (string form).
#[cfg(feature = "json")]
pub fn encode_json(program: &Program) -> Result<String, WireError> {
    crate::json::encode(program)
}

/// Decode a JSON-encoded [`Program`].
#[cfg(feature = "json")]
pub fn decode_json(s: &str) -> Result<Program, WireError> {
    crate::json::decode(s)
}

/// BLAKE3 content hash over the canonical postcard body of a [`Program`].
///
/// This is the recommended identity for migration keys, plan-cache keys,
/// and IoT idempotency tokens.
#[cfg(all(feature = "postcard", feature = "hash"))]
pub fn content_hash(program: &Program) -> Result<crate::hash::Digest, WireError> {
    crate::hash::content_hash(program)
}

extern crate alloc;
