//! BLAKE3 content hash of canonical wire bodies.
//!
//! The hash is computed over the **body** (excluding the [`crate::WireHeader`])
//! so a payload retains the same identity even if a future header version
//! is prepended for transport.

use alloc::string::String;

extern crate alloc;

/// 32-byte BLAKE3 digest.
pub type Digest = [u8; 32];

/// Hash an arbitrary byte body.
pub fn hash_bytes(body: &[u8]) -> Digest {
    *::blake3::hash(body).as_bytes()
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
pub fn hex(digest: &Digest) -> String {
    let mut out = String::with_capacity(64);
    for byte in digest {
        let hi = byte >> 4;
        let lo = byte & 0xF;
        out.push(nibble(hi));
        out.push(nibble(lo));
    }
    out
}

// `n` is in `0..=15` by the match arms, so `b'0'+n` and `b'a'+n-10` cannot
// overflow `u8`.
#[allow(clippy::arithmetic_side_effects)]
fn nibble(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'a' + n - 10) as char,
        _ => unreachable!(),
    }
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
