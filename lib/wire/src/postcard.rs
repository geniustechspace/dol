//! Postcard binary codec helpers.
//!
//! These helpers wrap the core [`postcard`] crate with the DOL [`crate::WireHeader`]
//! envelope. They operate on any `serde::Serialize` payload; downstream callers
//! pick the payload type.
//!
//! Only a `Serialize`-side `encode` is exposed: v2 wire-in goes through
//! [`crate::Decode`] exclusively (see [`crate::program::decode`] for the
//! `Program`-typed entry point).
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

extern crate alloc;
