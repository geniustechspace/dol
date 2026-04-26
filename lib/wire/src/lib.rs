//! # `dol-wire` — canonical codecs and content hashing
//!
//! Defines the stable wire envelope for any DOL payload: an 8-byte header
//! ([`WireHeader`]) followed by a body. The body's encoding is selected per
//! call (postcard for IoT, JSON for debugging) and is opaque to the header
//! layer.
//!
//! Every payload is prefixed with a [`WireHeader`] containing a stable
//! [`WireSchemaVersion`]. Additive changes bump `minor`; renames or removals
//! bump `major` and require a migration shim here.
//!
//! Content hashing (BLAKE3 over the canonical body bytes) gives every
//! payload a stable identifier reusable as a migration key, plan-cache key,
//! or IoT idempotency token.
//!
//! ## Typed `Program` codecs
//!
//! With the `program` feature (auto-enabled by `postcard` and `json`), this
//! crate exposes typed helpers in [`mod@program`] that target
//! [`dol_ir::Program`] directly:
//!
//! - [`program::encode_postcard`] / [`program::decode_postcard`]
//! - [`program::encode_json`] / [`program::decode_json`]
//! - [`program::content_hash`] — canonical BLAKE3 of the postcard body.
//!
//! The lower-level, payload-agnostic helpers in
//! [`mod@postcard`] and [`mod@json`] remain available for callers that
//! want to encode their own `Serialize` payloads behind the same envelope.

#![deny(unsafe_code)]
#![warn(missing_docs)]

mod header;
pub use header::{CURRENT_VERSION, WireHeader, WireSchemaVersion};

#[cfg(feature = "hash")]
pub mod hash;
#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "postcard")]
pub mod postcard;
#[cfg(any(feature = "postcard", feature = "json"))]
pub mod program;

/// Errors that can arise during wire encode/decode.
#[derive(Debug)]
pub enum WireError {
    /// Wire payload had the wrong magic prefix.
    BadMagic,
    /// Wire payload schema version is incompatible with this build.
    VersionMismatch {
        /// Version found in the header.
        found: WireSchemaVersion,
        /// Version this build supports.
        expected: WireSchemaVersion,
    },
    /// Underlying codec produced an error.
    Codec(alloc::string::String),
}

impl core::fmt::Display for WireError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BadMagic => f.write_str("dol-wire: bad magic"),
            Self::VersionMismatch { found, expected } => {
                write!(
                    f,
                    "dol-wire: version mismatch (found {found:?}, expected {expected:?})"
                )
            }
            Self::Codec(s) => write!(f, "dol-wire: codec error: {s}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for WireError {}

/// Prepend the current [`WireHeader`] to `body`, producing a wire-ready
/// payload. The body is treated as opaque bytes; callers choose how it was
/// encoded (postcard / json / …).
pub fn frame(body: &[u8]) -> alloc::vec::Vec<u8> {
    let header = WireHeader::current().to_bytes();
    let mut out = alloc::vec::Vec::with_capacity(header.len() + body.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(body);
    out
}

/// Verify the [`WireHeader`] of `bytes` and return the body slice. Returns a
/// [`WireError`] if the header is missing, mis-magic, or version-incompatible.
pub fn unframe(bytes: &[u8]) -> Result<&[u8], WireError> {
    if bytes.len() < 8 {
        return Err(WireError::BadMagic);
    }
    let mut header_buf = [0u8; 8];
    header_buf.copy_from_slice(&bytes[..8]);
    let _ = WireHeader::from_bytes(header_buf)?;
    Ok(&bytes[8..])
}

extern crate alloc;
