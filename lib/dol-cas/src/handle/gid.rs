//! [`Gid<Tag>`] — BLAKE3-256 global content address.
//!
//! Cross-process stable. 32 bytes. Used for signed manifests, wire
//! envelope `payload_hash`, and tamper-evident program identifiers.

use core::{fmt, hash::Hash, marker::PhantomData};

/// BLAKE3-256 global content address. Cross-process stable; the
/// canonical identity for signed manifests and wire envelopes
/// ([`dol_core::config::HashStrategy::CryptoFull`]).
///
/// `Gid<Tag>` is `#[repr(C)]` so its 32-byte raw representation is
/// FFI-stable.
#[repr(C)]
pub struct Gid<Tag: ?Sized> {
    raw: [u8; 32],
    _marker: PhantomData<fn() -> Tag>,
}

impl<Tag: ?Sized> Gid<Tag> {
    /// Wrap a raw 32-byte digest. Use [`dol_core::hash::content256`]
    /// to produce the bytes.
    #[inline]
    #[must_use]
    pub const fn from_bytes(raw: [u8; 32]) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    /// Borrow the underlying digest bytes.
    #[inline]
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.raw
    }

    /// Move out the underlying digest bytes.
    #[inline]
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.raw
    }
}

// ─── Hand-written trait impls (no Tag bound). ────────────────────────────────

impl<Tag: ?Sized> Clone for Gid<Tag> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Tag: ?Sized> Copy for Gid<Tag> {}

impl<Tag: ?Sized> PartialEq for Gid<Tag> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<Tag: ?Sized> Eq for Gid<Tag> {}

impl<Tag: ?Sized> PartialOrd for Gid<Tag> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<Tag: ?Sized> Ord for Gid<Tag> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.raw.cmp(&other.raw)
    }
}

impl<Tag: ?Sized> Hash for Gid<Tag> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<Tag: ?Sized> fmt::Debug for Gid<Tag> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Gid({:02x}{:02x}{:02x}{:02x}…)",
            self.raw[0], self.raw[1], self.raw[2], self.raw[3]
        )
    }
}

#[cfg(feature = "serde")]
impl<Tag: ?Sized> serde::Serialize for Gid<Tag> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.raw.serialize(s)
    }
}
// `Deserialize` intentionally omitted — wire-in goes through
// `dol_wire::Decode`.
