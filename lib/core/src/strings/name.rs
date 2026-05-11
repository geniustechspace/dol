//! Compact string-backed names used across `dol-core`.
//!
//! [`Name`] is the default concrete name representation for APIs that need
//! store-neutral identifiers such as function names, operator names, field
//! names, path segments, policy names, and similar symbolic labels.
//!
//! It supports two storage modes:
//!
//! - [`Name::Static`] for compile-time-known names with zero allocation.
//! - [`Name::Owned`] for runtime/custom names stored as `Box<str>`.
//!
//! Equality, ordering, hashing, display, and serialization are all defined in
//! terms of the string content, not the storage variant.
//!
//! # Wire policy
//!
//! [`Name`] implements `Serialize` for diagnostics, debug dumps, JSON exports,
//! and other outbound representations.
//!
//! It intentionally does **not** implement `Deserialize`. In v2, wire input
//! must go through `dol_wire::Decode`, where string length and allocation are
//! budget-checked.

use alloc::{boxed::Box, string::String};

use core::{borrow::Borrow, cmp::Ordering, fmt, hash::Hash};

/// A compact symbolic name.
///
/// Static names are stored without heap allocation. Runtime/custom names are
/// copied into an owned `Box<str>`.
///
/// Comparison and hashing operate on string content, so these are equivalent:
///
/// ```rust
/// use dol_core::name::Name;
///
/// let a = Name::Static("email");
/// let b = Name::owned("email");
///
/// assert_eq!(a, b);
/// assert_eq!(a.as_str(), "email");
/// ```
#[derive(Debug, Clone)]
pub enum Name {
    /// A compile-time-known name stored without heap allocation.
    Static(&'static str),

    /// A runtime/custom name stored on the heap.
    Owned(Box<str>),
}

impl Name {
    /// Creates an owned name from a runtime string.
    ///
    /// This always uses the owned representation. Prefer [`Name::Static`] or
    /// `Name::from("literal")` for compile-time-known names.
    #[inline]
    pub fn owned(s: impl Into<Box<str>>) -> Self {
        Self::Owned(s.into())
    }

    /// Returns the string content of this name.
    #[inline]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Static(s) => s,
            Self::Owned(s) => s,
        }
    }

    /// Returns `true` when this name is backed by a static string.
    #[inline]
    pub const fn is_static(&self) -> bool {
        matches!(self, Self::Static(_))
    }

    /// Returns `true` when this name is backed by an owned heap string.
    #[inline]
    pub const fn is_owned(&self) -> bool {
        matches!(self, Self::Owned(_))
    }
}

impl AsRef<str> for Name {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for Name {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq for Name {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<str> for Name {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Name {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl Eq for Name {}

impl PartialOrd for Name {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Name {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for Name {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl fmt::Display for Name {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&'static str> for Name {
    #[inline]
    fn from(s: &'static str) -> Self {
        Self::Static(s)
    }
}

impl From<Box<str>> for Name {
    #[inline]
    fn from(s: Box<str>) -> Self {
        Self::Owned(s)
    }
}

impl From<String> for Name {
    #[inline]
    fn from(s: String) -> Self {
        Self::Owned(s.into_boxed_str())
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Name {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

// `Deserialize` is intentionally not implemented.
// Wire-in goes through `dol_wire::Decode`, where string length and allocation
// are budget-checked.
