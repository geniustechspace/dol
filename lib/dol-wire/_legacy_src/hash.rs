//! BLAKE3 content hash of canonical wire bodies.
//!
//! The hash is computed over the **body** (excluding the [`crate::WireHeader`])
//! so a payload retains the same identity even if a future header version
//! is prepended for transport.
//!
//! All hashing is delegated to the [`dol_core::hash`] chokepoint — the
//! only crate in the workspace allowed to depend on `blake3` directly.

extern crate alloc;

use alloc::string::String;

/// 32-byte BLAKE3 digest, re-exported from [`dol_core::hash`].
pub type Digest = ::dol_core::hash::Digest256;

/// Hash an arbitrary byte body.
#[must_use]
pub fn hash_bytes(body: &[u8]) -> Digest {
    ::dol_core::hash::hash256(body)
}

/// Hash a postcard-encoded payload directly.
///
/// Requires the `postcard` feature.
#[cfg(feature = "postcard")]
pub fn content_hash<T: serde::Serialize>(payload: &T) -> Result<Digest, crate::WireError> {
    let body = ::postcard::to_allocvec(payload)
        .map_err(|e| crate::WireError::Codec(alloc::format!("{e}")))?;
    Ok(hash_bytes(&body))
}

/// Hex-encode a [`Digest`] as a 64-character lowercase string.
#[must_use]
pub fn hex(digest: &Digest) -> String {
    ::dol_core::hash::hex256(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable() {
        let a = hash_bytes(b"hello");
        let b = hash_bytes(b"hello");
        assert_eq!(a, b);
        assert_eq!(hex(&a).len(), 64);
    }
}
