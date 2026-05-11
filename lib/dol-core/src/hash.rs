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

/// Domain-separated BLAKE3 hasher used by DOL content addressing.
///
/// All BLAKE3 usage stays inside `dol_core::hash`.
#[allow(missing_docs)] // M0: doc backfill follows in M1 hash-module rewrite.
pub struct DomainSeparatedHasher {
    inner: blake3::Hasher,
}

#[allow(missing_docs)] // M0: doc backfill follows in M1 hash-module rewrite.
impl DomainSeparatedHasher {
    pub fn new(domain: &[u8]) -> Self {
        let mut inner = blake3::Hasher::new();
        inner.update(b"DOL-HASH-DOMAIN");
        inner.update(&(domain.len() as u64).to_le_bytes());
        inner.update(domain);
        Self { inner }
    }

    pub fn update(&mut self, bytes: &[u8]) {
        self.inner.update(bytes);
    }

    pub fn update_u16(&mut self, value: u16) {
        self.update(&value.to_le_bytes());
    }

    pub fn update_u32(&mut self, value: u32) {
        self.update(&value.to_le_bytes());
    }

    pub fn update_u64(&mut self, value: u64) {
        self.update(&value.to_le_bytes());
    }

    pub fn finalize_32(self) -> [u8; 32] {
        *self.inner.finalize().as_bytes()
    }
}

#[allow(missing_docs)] // M0: doc backfill follows in M1 hash-module rewrite.
pub fn hash64(domain: &[u8], bytes: &[u8]) -> [u8; 8] {
    let mut hasher = DomainSeparatedHasher::new(domain);
    hasher.update(bytes);
    let digest = hasher.finalize_32();
    let mut out = [0u8; 8];
    out.copy_from_slice(&digest[..8]);
    out
}

// The previous `hash128`/`hash256` domain-separated helpers below
// collided in name with the un-domain-separated truncations defined
// earlier in this file (lines ~95/101). They were never simultaneously
// callable. Gated out during the M0 scaffold; the M1 rewrite will pick
// the canonical names per `dol-rewrite-plan-v2.md` §6.3.
#[cfg(any())]
pub fn hash128(domain: &[u8], bytes: &[u8]) -> [u8; 16] {
    let mut hasher = DomainSeparatedHasher::new(domain);
    hasher.update(bytes);
    let digest = hasher.finalize_32();
    let mut out = [0u8; 16];
    out.copy_from_slice(&digest[..16]);
    out
}

#[cfg(any())]
pub fn hash256(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = DomainSeparatedHasher::new(domain);
    hasher.update(bytes);
    hasher.finalize_32()
}

// ──────────────────────────────────────────────────────────────────────────
// Canonical v2 API (per `dol-rewrite-plan-v2.md` §6.3).
//
// The names below are the single chokepoint that every other crate is
// expected to call. They split cleanly into two families:
//
// | family   | backing       | property                                  |
// |----------|---------------|-------------------------------------------|
// | `fast*`  | xxHash3       | non-cryptographic; in-process dedup only  |
// | `content*`| BLAKE3       | cryptographic; cross-process stable       |
//
// The legacy `hash32` / `hash128` / `hash256` and `Digest{32,128,256}`
// type aliases above are kept until M2 migrates the remaining caller
// (`strings::interner`) to `content128`. After that they are removed.
// ──────────────────────────────────────────────────────────────────────────

/// Fast non-cryptographic 64-bit hash. xxHash3 with seed `0`.
///
/// **In-process dedup only.** Two different processes may compute
/// different values for the same input (xxHash3 itself is deterministic
/// across processes today, but callers must not rely on that — the
/// strategy is allowed to change to a seeded variant by configuration).
/// Do **not** use this as a wire-stable identifier.
#[must_use]
#[inline]
pub fn fast64(bytes: &[u8]) -> u64 {
    xxhash_rust::xxh3::xxh3_64(bytes)
}

/// Fast non-cryptographic 64-bit hash with caller-supplied seed.
///
/// The seed is the same value stored in [`crate::config::PoolConfig::hash_seed`]
/// — picking a random seed per process is the standard defence against
/// algorithmic-complexity attacks on the pool's hash buckets.
#[must_use]
#[inline]
pub fn fast64_seeded(bytes: &[u8], seed: u64) -> u64 {
    xxhash_rust::xxh3::xxh3_64_with_seed(bytes, seed)
}

/// Fast non-cryptographic 128-bit hash. xxHash3 with seed `0`.
///
/// Same caveats as [`fast64`]. Used internally by [`crate::config::PoolConfig`]
/// sharding when 64 bits of fingerprint are insufficient.
#[must_use]
#[inline]
pub fn fast128(bytes: &[u8]) -> u128 {
    xxhash_rust::xxh3::xxh3_128(bytes)
}

/// Cryptographic 128-bit content address. BLAKE3 truncated to its
/// leading 16 bytes. **Cross-process stable.**
///
/// This is the canonical Cid backing in v2: identical inputs produce
/// identical bytes on every machine, every build of every crate, every
/// process. Suitable for `Cid` / `StringPool` content addressing.
#[must_use]
#[inline]
pub fn content128(bytes: &[u8]) -> [u8; 16] {
    let full = *blake3::hash(bytes).as_bytes();
    let mut out = [0u8; 16];
    out.copy_from_slice(&full[..16]);
    out
}

/// Cryptographic 256-bit content address. Full BLAKE3 output.
///
/// Used for `Gid`, signed manifests, and tamper-evident wire envelope
/// `payload_hash` per the v2 wire spec. Cross-process stable.
#[must_use]
#[inline]
pub fn content256(bytes: &[u8]) -> [u8; 32] {
    *blake3::hash(bytes).as_bytes()
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

    // ── v2 canonical API tests (per `dol-rewrite-plan-v2.md` §6.8). ──

    #[test]
    fn fast64_same_input_same_output() {
        let a = fast64(b"hello");
        let b = fast64(b"hello");
        assert_eq!(a, b);
    }

    #[test]
    fn fast64_different_inputs_different_output() {
        let a = fast64(b"hello");
        let b = fast64(b"world");
        assert_ne!(a, b);
    }

    #[test]
    fn fast64_seeded_changes_with_seed() {
        let a = fast64_seeded(b"hello", 0);
        let b = fast64_seeded(b"hello", 1);
        assert_ne!(a, b);
        // Same seed → same output.
        assert_eq!(fast64_seeded(b"hello", 42), fast64_seeded(b"hello", 42));
    }

    #[test]
    fn fast128_same_input_same_output() {
        assert_eq!(fast128(b"abc"), fast128(b"abc"));
        assert_ne!(fast128(b"abc"), fast128(b"abd"));
    }

    #[test]
    fn content128_same_input_same_output() {
        assert_eq!(content128(b"hello"), content128(b"hello"));
        assert_ne!(content128(b"hello"), content128(b"world"));
    }

    #[test]
    fn content128_is_prefix_of_content256() {
        let body = b"the quick brown fox";
        let d128 = content128(body);
        let d256 = content256(body);
        assert_eq!(&d256[..16], &d128[..]);
    }

    #[test]
    fn content256_matches_blake3_iv_for_empty() {
        // Cross-checks with the legacy hash256 helper.
        assert_eq!(content256(b""), hash256(b""));
    }
}
