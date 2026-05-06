//! Content-hash chokepoint — the only place in the workspace allowed to
//! call into [`blake3`] directly.
//!
//! v2 invariant (per `docs/v2_plan.md` §25): every consumer that needs a
//! cryptographic-grade hash uses [`Hasher`] / [`hash32`] / [`hash128`] /
//! [`hash256`] from this module, never `blake3::*` directly. That keeps
//! the underlying primitive swappable in one place if we ever need to
//! migrate (e.g. to a SIMD-accelerated successor or a hardware-rooted
//! KDF).
//!
//! # Output sizes
//!
//! BLAKE3 is an extendable-output function; we expose three fixed-size
//! truncations. All are prefixes of the same 256-bit digest, so
//! `hash32(b)`, `hash128(b)`, and `hash256(b)` are byte-prefix-consistent
//! for the same input.
//!
//! | function    | output                | typical use                              |
//! | ----------- | --------------------- | ---------------------------------------- |
//! | [`hash32`]  | [`Digest32`]  ( 4 B)  | short ids, bucketing, niche-friendly tags |
//! | [`hash128`] | [`Digest128`] (16 B)  | content-addressing where 256 b is overkill |
//! | [`hash256`] | [`Digest256`] (32 B)  | wire-frame `payload_hash`, signed manifests |
//!
//! # Feature gating
//!
//! This module is behind the `hash` cargo feature (default-off, opt-in
//! per the v2 "no_std + alloc minimal default" invariant). It enables
//! the `blake3` dependency.

use core::fmt;

/// 32-bit (4-byte) BLAKE3-derived digest.
pub type Digest32 = [u8; 4];

/// 128-bit (16-byte) BLAKE3-derived digest.
pub type Digest128 = [u8; 16];

/// 256-bit (32-byte) full BLAKE3 digest.
pub type Digest256 = [u8; 32];

/// Streaming BLAKE3 hasher.
///
/// Newtype wrapping [`blake3::Hasher`] so call sites never reference the
/// underlying crate. Use [`Hasher::finalize256`] / [`Hasher::finalize128`]
/// / [`Hasher::finalize32`] to extract a fixed-size digest.
#[derive(Clone, Default)]
pub struct Hasher(blake3::Hasher);

impl Hasher {
    /// Create a new hasher in the default (unkeyed) configuration.
    #[must_use]
    pub fn new() -> Self {
        Self(blake3::Hasher::new())
    }

    /// Feed `bytes` into the hasher. Returns `&mut self` for chaining.
    pub fn update(&mut self, bytes: &[u8]) -> &mut Self {
        self.0.update(bytes);
        self
    }

    /// Finalize and return the full 256-bit digest.
    #[must_use]
    pub fn finalize256(&self) -> Digest256 {
        *self.0.finalize().as_bytes()
    }

    /// Finalize and return the leading 128 bits of the digest.
    #[must_use]
    pub fn finalize128(&self) -> Digest128 {
        let full = self.finalize256();
        let mut out = [0u8; 16];
        out.copy_from_slice(&full[..16]);
        out
    }

    /// Finalize and return the leading 32 bits of the digest.
    #[must_use]
    pub fn finalize32(&self) -> Digest32 {
        let full = self.finalize256();
        let mut out = [0u8; 4];
        out.copy_from_slice(&full[..4]);
        out
    }
}

impl fmt::Debug for Hasher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Hasher").finish_non_exhaustive()
    }
}

/// One-shot 256-bit hash. Equivalent to `Hasher::new().update(bytes).finalize256()`.
#[must_use]
pub fn hash256(bytes: &[u8]) -> Digest256 {
    *blake3::hash(bytes).as_bytes()
}

/// One-shot 128-bit hash (leading 16 bytes of [`hash256`]).
#[must_use]
pub fn hash128(bytes: &[u8]) -> Digest128 {
    let full = hash256(bytes);
    let mut out = [0u8; 16];
    out.copy_from_slice(&full[..16]);
    out
}

/// One-shot 32-bit hash (leading 4 bytes of [`hash256`]).
#[must_use]
pub fn hash32(bytes: &[u8]) -> Digest32 {
    let full = hash256(bytes);
    let mut out = [0u8; 4];
    out.copy_from_slice(&full[..4]);
    out
}

/// Lowercase hex encoding of a 256-bit digest (64 chars).
#[must_use]
pub fn hex256(digest: &Digest256) -> alloc::string::String {
    hex_encode(digest)
}

/// Lowercase hex encoding of a 128-bit digest (32 chars).
#[must_use]
pub fn hex128(digest: &Digest128) -> alloc::string::String {
    hex_encode(digest)
}

/// Lowercase hex encoding of a 32-bit digest (8 chars).
#[must_use]
pub fn hex32(digest: &Digest32) -> alloc::string::String {
    hex_encode(digest)
}

fn hex_encode(bytes: &[u8]) -> alloc::string::String {
    use alloc::string::String;
    // Each byte expands to two hex nibbles. `bytes.len()` is bounded by
    // the caller-supplied digest size (≤ 32) so the multiplication cannot
    // overflow.
    #[allow(clippy::arithmetic_side_effects)]
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        // `byte >> 4` and `byte & 0xF` are both in 0..=15.
        out.push(nibble(byte >> 4));
        out.push(nibble(byte & 0xF));
    }
    out
}

// `n` is in 0..=15 by the match arms below, so `b'0' + n` and
// `b'a' + (n - 10)` cannot overflow `u8`.
#[allow(clippy::arithmetic_side_effects)]
const fn nibble(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'a' + n - 10) as char,
        // `n` is masked to 0..=15 by the only callers; this arm is
        // unreachable in practice. Use a const-fold-friendly fallback
        // rather than a panic so this stays panic-free.
        _ => '?',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncations_are_prefix_consistent() {
        let body = b"the quick brown fox";
        let d256 = hash256(body);
        let d128 = hash128(body);
        let d32 = hash32(body);
        assert_eq!(&d256[..16], &d128[..]);
        assert_eq!(&d256[..4], &d32[..]);
    }

    #[test]
    fn streaming_matches_oneshot() {
        let body = b"hello, world";
        let oneshot = hash256(body);

        let mut h = Hasher::new();
        h.update(b"hello, ").update(b"world");
        let streamed = h.finalize256();

        assert_eq!(oneshot, streamed);
    }

    #[test]
    fn finalize_truncations_are_consistent() {
        let mut h = Hasher::new();
        h.update(b"abc");
        let d256 = h.finalize256();
        let d128 = h.finalize128();
        let d32 = h.finalize32();
        assert_eq!(&d256[..16], &d128[..]);
        assert_eq!(&d256[..4], &d32[..]);
    }

    #[test]
    fn hex_encodings() {
        let d32: Digest32 = [0xde, 0xad, 0xbe, 0xef];
        assert_eq!(hex32(&d32), "deadbeef");
        let d128: Digest128 = [0xff; 16];
        assert_eq!(hex128(&d128).len(), 32);
        let d256: Digest256 = [0; 32];
        assert_eq!(hex256(&d256), "0".repeat(64));
    }

    #[test]
    fn empty_input_is_blake3_iv() {
        // Sanity: BLAKE3 of the empty string starts with these bytes.
        // (Spot-check first 8 bytes of the spec's IV digest.)
        let d = hash256(b"");
        assert_eq!(d[..4], [0xaf, 0x13, 0x49, 0xb9]);
    }
}
