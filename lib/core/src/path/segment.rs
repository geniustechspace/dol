//! Segment abstraction for [`Path`](super::Path).
//!
//! [`PathSegment`] lets a path store either inline string-backed names or
//! compact opaque IDs that require an external resolver.
//!
//! The default segment type is [`crate::strings::Name`], which resolves without
//! context. Interner-backed IDs such as `StrId` can implement this trait in
//! their owning crate and use the interner as the resolver.

use core::{fmt, hash::Hash};

use crate::strings::{Interner, Name, StrId};

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
}

impl PathSegment for Name {
    type Resolver = ();

    #[inline]
    fn resolve<'a>(&'a self, _: &'a ()) -> &'a str {
        self.as_str()
    }
}

impl PathSegment for StrId {
    type Resolver = Interner;

    #[inline]
    fn resolve<'a>(&'a self, resolver: &'a Interner) -> &'a str {
        resolver.get(*self)
    }
}
