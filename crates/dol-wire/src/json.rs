//! JSON codec helpers.
//!
//! The JSON envelope is intentionally human-friendly: a top-level object
//! with `"magic"`, `"version"`, and `"payload"` fields. Useful for debugging
//! and language-bridge interop; not intended for resource-constrained
//! on-device use.

use alloc::string::String;
use alloc::vec::Vec;

use crate::{CURRENT_VERSION, WireError, WireSchemaVersion};

#[derive(serde::Serialize)]
struct EncEnvelope<'a, T: serde::Serialize> {
    magic: &'static str,
    version: [u16; 2],
    payload: &'a T,
}

#[derive(serde::Deserialize)]
struct DecEnvelope<T> {
    #[serde(default)]
    magic: alloc::string::String,
    version: [u16; 2],
    payload: T,
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

/// Decode a JSON-encoded envelope into `T`, verifying magic + version.
pub fn decode<T: for<'de> serde::Deserialize<'de>>(s: &str) -> Result<T, WireError> {
    let env: DecEnvelope<T> =
        serde_json::from_str(s).map_err(|e| WireError::Codec(alloc::format!("{e}")))?;
    if env.magic != "DOL" {
        return Err(WireError::BadMagic);
    }
    let v = WireSchemaVersion::new(env.version[0], env.version[1]);
    if !CURRENT_VERSION.accepts(v) {
        return Err(WireError::VersionMismatch {
            found: v,
            expected: CURRENT_VERSION,
        });
    }
    Ok(env.payload)
}

extern crate alloc;
