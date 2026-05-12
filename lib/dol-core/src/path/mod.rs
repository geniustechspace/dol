//! Structured, store-neutral paths.
//!
//! A [`Path`] identifies an addressable target and, optionally, a nested field
//! within that target.
//!
//! It has three parts:
//!
//! | Part        | Meaning                                    | Required |
//! |-------------|--------------------------------------------|----------|
//! | `namespace` | Optional scope / authority token           | no       |
//! | `target`    | Addressable resource token                 | yes      |
//! | `field`     | Optional chain into the addressed resource | no       |
//!
//! Compound target identities should be flattened by the caller before
//! constructing a path. For example, a REST endpoint may use `"users/profile"`
//! as the target, while a database entity may use `"users"`.

mod segment;

use smallvec::SmallVec;

use crate::strings::Name;

pub use segment::PathSegment;
#[cfg(feature = "hash")]
pub use segment::StringResolver;

/// A structured path to a target and optional nested field.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Path<N: PathSegment = Name> {
    namespace: Option<N>,
    target: N,
    field: SmallVec<[N; 2]>,
}

impl<N: PathSegment> Path<N> {
    /// Creates a path from an already-typed segment.
    ///
    /// This constructor is used for non-default path segment types such as
    /// interned IDs. For ordinary string-backed paths, use [`Path::new`].
    #[inline]
    pub fn from_segment(target: N) -> Self {
        Self {
            namespace: None,
            target,
            field: SmallVec::new(),
        }
    }

    /// Sets or replaces the namespace.
    ///
    /// The namespace is a single scope/authority token.
    #[inline]
    pub fn namespace(mut self, namespace: impl Into<N>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Appends a segment to the field chain.
    #[inline]
    pub fn get(mut self, segment: impl Into<N>) -> Self {
        self.field.push(segment.into());
        self
    }

    /// Returns `true` when a namespace is present.
    #[inline]
    pub fn has_namespace(&self) -> bool {
        self.namespace.is_some()
    }

    /// Returns `true` when at least one field segment is present.
    #[inline]
    pub fn has_fields(&self) -> bool {
        !self.field.is_empty()
    }

    /// Returns the number of field segments.
    #[inline]
    pub fn field_len(&self) -> usize {
        self.field.len()
    }

    /// Returns the namespace segment, if present.
    #[inline]
    pub fn namespace_segment(&self) -> Option<&N> {
        self.namespace.as_ref()
    }

    /// Returns the target segment.
    #[inline]
    pub fn target_segment(&self) -> &N {
        &self.target
    }

    /// Returns the field segment slice.
    #[inline]
    pub fn field_segments(&self) -> &[N] {
        &self.field
    }

    /// Resolves the namespace using the provided segment resolver.
    #[inline]
    pub fn resolve_namespace<'a>(&'a self, resolver: &'a N::Resolver) -> Option<&'a str> {
        self.namespace.as_ref().map(|n| n.resolve(resolver))
    }

    /// Resolves the target using the provided segment resolver.
    #[inline]
    pub fn resolve_target<'a>(&'a self, resolver: &'a N::Resolver) -> &'a str {
        self.target.resolve(resolver)
    }

    /// Resolves field segments using the provided segment resolver.
    #[inline]
    pub fn resolve_fields<'a>(
        &'a self,
        resolver: &'a N::Resolver,
    ) -> impl Iterator<Item = &'a str> + 'a {
        self.field.iter().map(move |n| n.resolve(resolver))
    }
}

impl Path<Name> {
    /// Creates a default string-backed path.
    ///
    /// This constructor is intentionally defined only for `Path<Name>` so that
    /// `Path::new("users")` infers cleanly even when other crates implement
    /// `From<&'static str>` for their own types.
    #[inline]
    pub fn new(target: impl Into<Name>) -> Self {
        Self {
            namespace: None,
            target: target.into(),
            field: SmallVec::new(),
        }
    }

    /// Returns the namespace string, if present.
    #[inline]
    pub fn namespace_str(&self) -> Option<&str> {
        self.namespace.as_ref().map(Name::as_str)
    }

    /// Returns the target string.
    #[inline]
    pub fn target_str(&self) -> &str {
        self.target.as_str()
    }

    /// Iterates over field segments as strings.
    #[inline]
    pub fn field_strs(&self) -> impl Iterator<Item = &str> {
        self.field.iter().map(Name::as_str)
    }

    /// Returns the single field segment, or `None` when there are zero or more
    /// than one field segments.
    #[inline]
    pub fn field_single(&self) -> Option<&str> {
        match self.field.as_slice() {
            [single] => Some(single.as_str()),
            _ => None,
        }
    }
}

/// Tests for [`Path`] and related types.
#[cfg(test)]
mod tests;
