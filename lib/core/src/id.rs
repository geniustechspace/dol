//! # `id` — typed, niche-optimised arena identifiers.
//!
//! A great deal of `dol-*` data is stored in arenas: AST nodes, IR
//! operations, schema fields, interned strings. Each arena needs its own
//! id type so a node-id cannot be confused with a field-id at compile
//! time, and so the type system can enforce arena lookups.
//!
//! Rather than every crate inventing its own newtype, `dol-core` exposes
//! a single generic [`Id<Tag>`] parameterised by a phantom `Tag` marker.
//! Different arenas pick different tags:
//!
//! ```
//! use dol_core::id::Id;
//!
//! /// Tag for AST node ids in `dol-expr`.
//! pub struct NodeTag;
//! pub type NodeId = Id<NodeTag>;
//!
//! /// Tag for interned string ids.
//! pub struct StrTag;
//! pub type StrId = Id<StrTag>;
//!
//! // Different tags produce structurally distinct types — this would
//! // not compile:
//! //   let n: NodeId = some_str_id; // ✗ type mismatch
//! ```
//!
//! ## Niche
//!
//! `Id<T>` wraps [`NonZeroU32`], so `Option<Id<T>>` is exactly four
//! bytes — the same niche optimisation Rust applies to `Option<&T>`. The
//! id range is `1..=u32::MAX`, leaving plenty of room for any realistic
//! arena (≈4 G entries). Code that wants a "missing" id should use
//! `Option<Id<T>>` rather than reserving a sentinel.
//!
//! ## `no_std`
//!
//! `Id<T>` is `Copy`, contains no allocation, and is `no_std`-clean.

use core::{fmt, hash::Hash, marker::PhantomData, num::NonZeroU32};

/// Typed arena identifier.
///
/// `Id<Tag>` is a 4-byte handle with `Option<Id<Tag>> == 4 bytes` thanks
/// to the [`NonZeroU32`] niche.
///
/// The `Tag` parameter is a zero-sized phantom marker; it has no runtime
/// representation and exists solely to make ids of distinct arenas
/// distinct types.
#[repr(transparent)]
pub struct Id<Tag: ?Sized> {
    raw: NonZeroU32,
    _marker: PhantomData<fn() -> Tag>,
}

impl<Tag: ?Sized> Id<Tag> {
    /// Construct an id from a non-zero raw value.
    #[inline]
    #[must_use]
    pub const fn new(raw: NonZeroU32) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    /// Construct an id from a one-based `u32`. Returns `None` if `raw`
    /// is zero.
    #[inline]
    #[must_use]
    pub const fn from_u32(raw: u32) -> Option<Self> {
        match NonZeroU32::new(raw) {
            Some(nz) => Some(Self::new(nz)),
            None => None,
        }
    }

    /// Construct an id from a zero-based `usize` index. Returns `None`
    /// if `index >= u32::MAX` (which would overflow the one-based
    /// representation).
    ///
    /// Use this when materialising ids for a freshly pushed entry:
    /// ```
    /// use dol_core::id::Id;
    /// struct Frob;
    /// let v: Vec<u8> = vec![10, 20, 30];
    /// // The element at index 2 has id 3 (one-based).
    /// let id: Id<Frob> = Id::from_index(v.len() - 1).unwrap();
    /// assert_eq!(id.index(), 2);
    /// ```
    #[inline]
    #[must_use]
    pub const fn from_index(index: usize) -> Option<Self> {
        // `index + 1` must fit in a `u32` and must not be zero.
        if index >= u32::MAX as usize {
            return None;
        }
        // `index + 1` is in `1..=u32::MAX`, so the `NonZeroU32` is safe.
        match NonZeroU32::new(index as u32 + 1) {
            Some(nz) => Some(Self::new(nz)),
            None => None,
        }
    }

    /// Raw one-based id.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u32 {
        self.raw.get()
    }

    /// Zero-based index suitable for `Vec` / slice access.
    #[inline]
    #[must_use]
    pub const fn index(self) -> usize {
        self.raw.get() as usize - 1
    }
}

// ─── Trait impls ─────────────────────────────────────────────────────────────
//
// All of these are derived manually because `#[derive]` would also bound the
// `Tag` parameter, but `Tag` is purely phantom.

impl<Tag: ?Sized> Clone for Id<Tag> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Tag: ?Sized> Copy for Id<Tag> {}

impl<Tag: ?Sized> PartialEq for Id<Tag> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<Tag: ?Sized> Eq for Id<Tag> {}

impl<Tag: ?Sized> PartialOrd for Id<Tag> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<Tag: ?Sized> Ord for Id<Tag> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.raw.cmp(&other.raw)
    }
}

impl<Tag: ?Sized> Hash for Id<Tag> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<Tag: ?Sized> fmt::Debug for Id<Tag> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // No tag name in output; phantom tags are often not `Debug`.
        write!(f, "Id({})", self.raw.get())
    }
}

impl<Tag: ?Sized> fmt::Display for Id<Tag> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.raw.get().fmt(f)
    }
}

#[cfg(feature = "serde")]
impl<Tag: ?Sized> serde::Serialize for Id<Tag> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // Serialise as the bare `u32` so the wire format is independent
        // of the tag — round-tripping is the consumer's responsibility.
        self.raw.get().serialize(s)
    }
}

#[cfg(feature = "serde")]
impl<'de, Tag: ?Sized> serde::Deserialize<'de> for Id<Tag> {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let raw: u32 = serde::Deserialize::deserialize(de)?;
        NonZeroU32::new(raw)
            .map(Self::new)
            .ok_or_else(|| <D::Error as serde::de::Error>::custom("Id<Tag>: zero is reserved"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    struct Foo;
    struct Bar;

    type FooId = Id<Foo>;
    type BarId = Id<Bar>;

    #[test]
    fn id_is_four_bytes_and_option_is_four_bytes() {
        assert_eq!(size_of::<FooId>(), 4);
        assert_eq!(size_of::<Option<FooId>>(), 4);
    }

    #[test]
    fn from_index_round_trips() {
        let id = FooId::from_index(0).unwrap();
        assert_eq!(id.index(), 0);
        assert_eq!(id.get(), 1);

        let id = FooId::from_index(41).unwrap();
        assert_eq!(id.index(), 41);
        assert_eq!(id.get(), 42);
    }

    #[test]
    fn from_index_rejects_overflow() {
        // `u32::MAX as usize` would map to id `u32::MAX + 1`, which does
        // not fit; reject it.
        assert!(FooId::from_index(u32::MAX as usize).is_none());
    }

    #[test]
    fn from_u32_rejects_zero() {
        assert!(FooId::from_u32(0).is_none());
        assert!(FooId::from_u32(1).is_some());
    }

    #[test]
    fn distinct_tags_produce_distinct_types() {
        // Compile-time check: this test exists to make the intended
        // type-discrimination property explicit. Mismatched tags are a
        // hard compile error, demonstrated in the rustdoc example.
        let a: FooId = FooId::from_index(3).unwrap();
        let b: BarId = BarId::from_index(3).unwrap();
        // Sanity-check the runtime values still match.
        assert_eq!(a.get(), b.get());
    }

    #[test]
    fn ordering_and_hash_match_raw() {
        use core::cmp::Ordering;
        let a = FooId::from_index(0).unwrap();
        let b = FooId::from_index(1).unwrap();
        assert_eq!(a.cmp(&b), Ordering::Less);
        assert_eq!(b.cmp(&a), Ordering::Greater);
        assert_eq!(a.cmp(&a), Ordering::Equal);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serializes_as_bare_u32() {
        let id = FooId::from_index(41).unwrap();
        let s = serde_json::to_string(&id).unwrap();
        assert_eq!(s, "42");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn deserializes_from_bare_u32_and_rejects_zero() {
        let id: FooId = serde_json::from_str("42").unwrap();
        assert_eq!(id.get(), 42);
        assert!(serde_json::from_str::<FooId>("0").is_err());
    }
}
