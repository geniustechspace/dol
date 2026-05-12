//! Segment abstraction for [`Path`](super::Path).
//!
//! [`PathSegment`] lets a path store either inline string-backed names or
//! compact opaque IDs that require an external resolver.
//!
//! The default segment type is [`crate::strings::Name`], which resolves without
//! context. Interner-backed IDs such as `StrId` resolve through any
//! pool that implements [`StringResolver`] — the legacy in-tree
//! [`crate::strings::Interner`] and `dol-cas`'s `StringPool` both qualify.

use core::{fmt, hash::Hash};

use crate::strings::Name;
#[cfg(feature = "hash")]
use crate::strings::{Interner, StrId};

/// A value that can be stored as a segment inside a [`Path`](super::Path).
///
/// Segment values must be cloneable, comparable, hashable, and debuggable.
/// Resolution to `&str` may require external context.
///
/// # Resolver contract
///
/// - Inline string-backed segments should use `Resolver = ()`.
/// - Interned or opaque ID segments should use the issuing interner/pool type.
///
/// This prevents code from accidentally resolving an interned path without the
/// resolver needed to make its IDs meaningful.
///
/// # Content identity
///
/// [`PathSegment::content_id`] returns the cross-process stable BLAKE3
/// content address of the segment, when one is available. `Name`
/// segments return `None` — they carry no stable identity. Pool-backed
/// segments (e.g. `Lid<StrTag>` in M2) override the default to return
/// the address lazily computed at intern time.
pub trait PathSegment: Clone + Eq + Hash + fmt::Debug {
    /// Context required to resolve this segment to a string.
    ///
    /// Use `()` when the segment stores the string directly.
    type Resolver: ?Sized;

    /// Resolves this segment into a string slice.
    ///
    /// The returned string is tied to both `self` and `resolver`, which supports
    /// both inline segments and resolver-owned storage.
    fn resolve<'a>(&'a self, resolver: &'a Self::Resolver) -> &'a str;

    /// Returns the cross-process stable BLAKE3-128 content address of
    /// this segment, if available.
    ///
    /// `Name` segments and other inline string types return `None` —
    /// they carry no stable identity beyond the bytes themselves.
    /// Pool-backed segments (e.g. `Lid<StrTag>` in M2) override this
    /// default to return the address lazily computed at intern time.
    ///
    /// The default impl is `None`, so existing segment types remain
    /// source-compatible.
    fn content_id(&self, _resolver: &Self::Resolver) -> Option<[u8; 16]> {
        None
    }
}

impl PathSegment for Name {
    type Resolver = ();

    #[inline]
    fn resolve<'a>(&'a self, _: &'a ()) -> &'a str {
        self.as_str()
    }
}

#[cfg(feature = "hash")]
impl PathSegment for StrId {
    type Resolver = dyn StringResolver + 'static;

    #[inline]
    fn resolve<'a>(&'a self, resolver: &'a (dyn StringResolver + 'static)) -> &'a str {
        // `unwrap_or("")` mirrors the plan §7.5 contract: an unknown
        // or stale handle resolves to the empty string rather than
        // panicking, so path resolution stays infallible.
        resolver.resolve_str(*self).unwrap_or("")
    }

    #[inline]
    fn content_id(&self, resolver: &(dyn StringResolver + 'static)) -> Option<[u8; 16]> {
        resolver.content_id_for(*self)
    }
}

/// Abstract resolver for [`StrId`] handles.
///
/// `dol-core` ships [`PathSegment for StrId`](PathSegment) with
/// `Resolver = dyn StringResolver`, so any pool that issues `StrId`s
/// (the legacy in-tree [`Interner`], `dol-cas`'s `StringPool`, or any
/// future bare-metal pool) can be passed as the resolver of a
/// `Path<StrId>`.
///
/// The trait is the **only** point at which dol-core admits an
/// outside-issued resolver into the `PathSegment` machinery; this is
/// what lets `dol-cas::StringPool` close the two-mode path bridge
/// (per `dol-rewrite-plan-v2.md` §7.5) without `dol-core` ever having
/// to know about `dol-cas`.
#[cfg(feature = "hash")]
pub trait StringResolver {
    /// Resolve `id` to its interned string slice.
    ///
    /// Returns `None` when `id` is unknown to this resolver. The
    /// `Path` machinery converts `None` to an empty string so that
    /// resolution stays infallible.
    fn resolve_str(&self, id: StrId) -> Option<&str>;

    /// Returns the cross-process stable BLAKE3-128 content address of
    /// the string referenced by `id`, when one is available.
    ///
    /// Default: `None` — pools without a stable identity simply
    /// inherit the `PathSegment::content_id` default.
    fn content_id_for(&self, _id: StrId) -> Option<[u8; 16]> {
        None
    }
}

/// The legacy in-tree [`Interner`] is itself a [`StringResolver`].
/// This keeps `Path<StrId>` callers that resolve against an
/// `Interner` source-compatible after the trait reshape.
#[cfg(feature = "hash")]
impl StringResolver for Interner {
    #[inline]
    fn resolve_str(&self, id: StrId) -> Option<&str> {
        // `Interner::get` returns `&str` directly — empty string when
        // the id is unknown — so we always have a slice to hand back.
        Some(self.get(id))
    }
}
