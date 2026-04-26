//! Postcard binary codec helpers.
//!
//! These helpers wrap the core [`postcard`] crate with the DOL [`crate::WireHeader`]
//! envelope. They operate on any `serde::Serialize` payload; downstream callers
//! pick the payload type (a [`dol_ir::Program`] once `dol-ir` is fully
//! serde-able, or a domain-specific subset in the meantime).
//!
//! Encoded layout:
//!
//! ```text
//! [ MAGIC (4) | VERSION (4) | postcard-encoded body ]
//! ```

use alloc::vec::Vec;

use crate::{WireError, WireHeader};

/// Encode `payload` to postcard bytes and prepend a [`WireHeader`].
pub fn encode<T: serde::Serialize>(payload: &T) -> Result<Vec<u8>, WireError> {
    let header = WireHeader::current().to_bytes();
    let body =
        ::postcard::to_allocvec(payload).map_err(|e| WireError::Codec(alloc::format!("{e}")))?;
    let mut out = Vec::with_capacity(header.len() + body.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(&body);
    Ok(out)
}

/// Verify the [`WireHeader`] and decode the postcard body into `T`.
pub fn decode<'de, T: serde::Deserialize<'de>>(bytes: &'de [u8]) -> Result<T, WireError> {
    let body = crate::unframe(bytes)?;
    ::postcard::from_bytes::<T>(body).map_err(|e| WireError::Codec(alloc::format!("{e}")))
}

extern crate alloc;
