//! JSON codec helpers.
//!
//! The JSON envelope is intentionally human-friendly: a top-level object
//! with `"magic"`, `"version"`, and `"payload"` fields. Useful for debugging
//! and language-bridge interop; not intended for resource-constrained
//! on-device use.
//!
//! Only a `Serialize`-side `encode` is exposed: v2 wire-in goes through
//! [`crate::Decode`] exclusively. JSON wire-in is intentionally absent
//! to keep the validating-decode invariant tight.

use alloc::string::String;
use alloc::vec::Vec;

use crate::{CURRENT_VERSION, WireError};

#[derive(serde::Serialize)]
struct EncEnvelope<'a, T: serde::Serialize> {
    magic: &'static str,
    version: [u16; 2],
    payload: &'a T,
}

/// Encode `payload` to a JSON string with the wire envelope.
pub fn encode<T: serde::Serialize>(payload: &T) -> Result<String, WireError> {
    let env = EncEnvelope {
        magic: "DOL",
        version: [CURRENT_VERSION.major, CURRENT_VERSION.minor],
        payload,
    };
    serde_json::to_string(&env).map_err(|e| WireError::Codec(alloc::format!("{e}")))
}

/// Encode `payload` to JSON bytes.
pub fn encode_bytes<T: serde::Serialize>(payload: &T) -> Result<Vec<u8>, WireError> {
    encode(payload).map(String::into_bytes)
}

extern crate alloc;
