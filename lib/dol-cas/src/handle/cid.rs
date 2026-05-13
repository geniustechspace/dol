//! [`Cid<Tag>`] — BLAKE3-128 content address.
//!
//! Cross-process stable. 16 bytes. The hashing primitive lives in
//! [`dol_core::hash::content128`]; `Cid` is the typed wrapper so two
//! pools' content addresses cannot accidentally be compared.

use core::{fmt, hash::Hash, marker::PhantomData};

/// BLAKE3-128 content address. Cross-process stable; suitable as a
/// wire-stable identifier when the upstream producer commits to the
/// same hashing strategy ([`dol_core::config::HashStrategy::Crypto`]).
///
/// `Cid<Tag>` is `#[repr(C)]` so its 16-byte raw representation is
/// FFI-stable and `bytemuck`-able. The `Tag` phantom keeps content
/// addresses of distinct content kinds distinct types.
#[repr(C)]
pub struct Cid<Tag: ?Sized> {
    raw: [u8; 16],
    _marker: PhantomData<fn() -> Tag>,
}

impl<Tag: ?Sized> Cid<Tag> {
    /// Wrap a raw 16-byte digest. Use [`dol_core::hash::content128`]
    /// to produce the bytes.
    #[inline]
    #[must_use]
    pub const fn from_bytes(raw: [u8; 16]) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    /// Borrow the underlying digest bytes.
    #[inline]
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.raw
    }

    /// Move out the underlying digest bytes.
    #[inline]
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 16] {
        self.raw
    }
}

// ─── Hand-written trait impls (no Tag bound). ────────────────────────────────

impl<Tag: ?Sized> Clone for Cid<Tag> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Tag: ?Sized> Copy for Cid<Tag> {}

impl<Tag: ?Sized> PartialEq for Cid<Tag> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<Tag: ?Sized> Eq for Cid<Tag> {}

impl<Tag: ?Sized> PartialOrd for Cid<Tag> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<Tag: ?Sized> Ord for Cid<Tag> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.raw.cmp(&other.raw)
    }
}

impl<Tag: ?Sized> Hash for Cid<Tag> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<Tag: ?Sized> fmt::Debug for Cid<Tag> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Hex preview of the first 4 bytes — full 16-byte hex is
        // available via [`Cid::as_bytes`] + an external hex encoder.
        write!(
            f,
            "Cid({:02x}{:02x}{:02x}{:02x}…)",
            self.raw[0], self.raw[1], self.raw[2], self.raw[3]
        )
    }
}

#[cfg(feature = "serde")]
impl<Tag: ?Sized> serde::Serialize for Cid<Tag> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // Serialise as a bare 16-byte array so the wire format is
        // independent of the tag.
        self.raw.serialize(s)
    }
}
// `Deserialize` intentionally omitted — wire-in goes through
// `dol_wire::Decode`.
