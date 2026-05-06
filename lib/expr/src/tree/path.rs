//! Hierarchical path expressions for field and source references.

use smallvec::SmallVec;

use super::compact_name::CompactName;

/// A non-empty path of identifier segments, stored inline for paths ≤ 3 deep.
///
/// `SmallVec<[CompactName; 3]>` means:
/// - `"age"` — 1 segment, zero heap allocation.
/// - `"users.email"` — 2 segments, zero heap allocation.
/// - `"profile.address.city"` — 3 segments, zero heap allocation.
/// - 4+ segments spill to the heap (uncommon).
///
/// # Examples
///
/// ```rust
/// use dol_expr::tree::PathExpr;
///
/// let p = PathExpr::one("email");
/// let q = PathExpr::from_segments(["profile", "address", "city"]);
/// let r = PathExpr::one("profile").push("address");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PathExpr {
    pub segments: SmallVec<[CompactName; 3]>,
}

impl PathExpr {
    /// Create a single-segment path from a `CompactName`-compatible value.
    ///
    /// String literals are **zero-alloc**: `PathExpr::one("age")` stores
    /// `CompactName::Static("age")` inline.
    pub fn one(name: impl Into<CompactName>) -> Self {
        let mut segments = SmallVec::new();
        segments.push(name.into());
        Self { segments }
    }

    /// Create a path from a runtime `&str` (always allocates one `Box<str>`).
    ///
    /// Use `PathExpr::one` with a string literal when the name is known at
    /// compile time.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(name: &str) -> Self {
        let mut segments = SmallVec::new();
        segments.push(CompactName::from_str(name));
        Self { segments }
    }

    /// Create a multi-segment path from an iterator of `CompactName`-compatible values.
    ///
    /// # Panics
    ///
    /// Panics if the iterator is empty. `PathExpr` is a non-empty path by
    /// construction; use [`PathExpr::try_from_segments`] for a fallible
    /// alternative that returns `None` on empty input.
    pub fn from_segments<I, S>(iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<CompactName>,
    {
        let segments: SmallVec<[CompactName; 3]> = iter.into_iter().map(Into::into).collect();
        assert!(
            !segments.is_empty(),
            "PathExpr must have at least one segment"
        );
        Self { segments }
    }

    /// Fallible variant of [`PathExpr::from_segments`] — returns `None` when
    /// the iterator is empty instead of panicking.
    pub fn try_from_segments<I, S>(iter: I) -> Option<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<CompactName>,
    {
        let segments: SmallVec<[CompactName; 3]> = iter.into_iter().map(Into::into).collect();
        if segments.is_empty() {
            None
        } else {
            Some(Self { segments })
        }
    }

    /// Append a segment, returning the extended path.
    pub fn push(mut self, segment: impl Into<CompactName>) -> Self {
        self.segments.push(segment.into());
        self
    }

    /// Push a segment from a runtime `&str`.
    pub fn push_str(mut self, segment: &str) -> Self {
        self.segments.push(CompactName::from_str(segment));
        self
    }

    /// Return the single segment string if this is a one-segment path.
    pub fn as_single(&self) -> Option<&str> {
        if self.segments.len() == 1 {
            Some(self.segments[0].as_str())
        } else {
            None
        }
    }

    /// Iterate over segment strings.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.segments.iter().map(|s| s.as_str())
    }

    /// Number of segments.
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Always false — a `PathExpr` is always non-empty by construction.
    pub fn is_empty(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn single_segment_zero_alloc() {
        let p = PathExpr::one("email");
        assert_eq!(p.segments.len(), 1);
        assert_eq!(p.segments[0].as_str(), "email");
        // Static variant — no heap allocation for the string content.
        assert!(matches!(p.segments[0], CompactName::Static("email")));
    }

    #[test]
    fn multi_segment_from_literals() {
        let p = PathExpr::from_segments(["profile", "address", "city"]);
        assert_eq!(p.len(), 3);
        let collected: Vec<&str> = p.iter().collect();
        assert_eq!(collected, ["profile", "address", "city"]);
    }

    #[test]
    fn push_extends_path() {
        let p = PathExpr::one("profile").push("address").push("city");
        assert_eq!(p.len(), 3);
    }

    #[test]
    fn as_single_on_one_segment() {
        assert_eq!(PathExpr::one("id").as_single(), Some("id"));
    }

    #[test]
    fn as_single_on_multi_segment() {
        assert_eq!(PathExpr::from_segments(["a", "b"]).as_single(), None);
    }

    #[test]
    fn equality_by_content() {
        let a = PathExpr::one("x");
        let b = PathExpr::from_str("x");
        assert_eq!(a, b);
    }
}
