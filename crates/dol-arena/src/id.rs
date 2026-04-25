use core::fmt;
use core::marker::PhantomData;

/// Sentinel "no id" value reusable by callers that need an `Option<Id<_>>`-free
/// representation. Equal to `u32::MAX`.
pub const NULL_ID: u32 = u32::MAX;

/// A typed index into an [`Arena<T>`](crate::Arena).
///
/// `T` is the payload type and `Tag` is an optional zero-sized marker used to
/// give two arenas storing the same `T` distinct id types. For most callers the
/// default `Tag = T` is exactly what you want.
///
/// `Id` is `Copy`, 4 bytes, and never points outside its owning arena.
#[repr(transparent)]
pub struct Id<T, Tag: ?Sized = T> {
    raw: u32,
    _t: PhantomData<fn() -> (T, *const Tag)>,
}

impl<T, Tag: ?Sized> Id<T, Tag> {
    /// Construct an [`Id`] from a raw 32-bit index. Callers are expected to
    /// only build ids returned by an [`Arena`](crate::Arena).
    #[inline]
    pub const fn from_raw(raw: u32) -> Self {
        Self {
            raw,
            _t: PhantomData,
        }
    }

    /// Get the raw 32-bit index.
    #[inline]
    pub const fn to_raw(self) -> u32 {
        self.raw
    }

    /// Returns `true` if this id equals [`NULL_ID`].
    #[inline]
    pub const fn is_null(self) -> bool {
        self.raw == NULL_ID
    }
}

impl<T, Tag: ?Sized> Clone for Id<T, Tag> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, Tag: ?Sized> Copy for Id<T, Tag> {}
impl<T, Tag: ?Sized> PartialEq for Id<T, Tag> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}
impl<T, Tag: ?Sized> Eq for Id<T, Tag> {}
impl<T, Tag: ?Sized> core::hash::Hash for Id<T, Tag> {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}
impl<T, Tag: ?Sized> PartialOrd for Id<T, Tag> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T, Tag: ?Sized> Ord for Id<T, Tag> {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.raw.cmp(&other.raw)
    }
}
impl<T, Tag: ?Sized> fmt::Debug for Id<T, Tag> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id<{}>({})", core::any::type_name::<T>(), self.raw)
    }
}

#[cfg(feature = "serde")]
mod _serde_id {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl<T, Tag: ?Sized> Serialize for Id<T, Tag> {
        fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
            self.raw.serialize(ser)
        }
    }
    impl<'de, T, Tag: ?Sized> Deserialize<'de> for Id<T, Tag> {
        fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
            u32::deserialize(de).map(Self::from_raw)
        }
    }
}

// ─── StrId ────────────────────────────────────────────────────────────────────

/// Compact string identifier: 24-bit index + 8-bit user-defined `kind` byte.
///
/// The `kind` byte is opaque to `dol-arena` — callers can use it to classify
/// strings (e.g. identifier vs literal) without a parallel side table.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StrId(u32);

impl StrId {
    /// Maximum representable index.
    pub const MAX_INDEX: u32 = 0x00FF_FFFF;

    /// Build a [`StrId`] from a 24-bit index and an 8-bit `kind`. Panics in
    /// debug if the index exceeds [`Self::MAX_INDEX`].
    #[inline]
    pub const fn new(index: u32, kind: u8) -> Self {
        debug_assert!(index <= Self::MAX_INDEX);
        Self(((kind as u32) << 24) | (index & Self::MAX_INDEX))
    }

    /// Get the 24-bit index portion.
    #[inline]
    pub const fn index(self) -> u32 {
        self.0 & Self::MAX_INDEX
    }

    /// Get the 8-bit user `kind` byte.
    #[inline]
    pub const fn kind(self) -> u8 {
        (self.0 >> 24) as u8
    }

    /// Get the raw packed `u32`.
    #[inline]
    pub const fn to_raw(self) -> u32 {
        self.0
    }

    /// Reconstruct from the raw packed `u32` form.
    #[inline]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }
}
