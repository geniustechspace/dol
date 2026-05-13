//! Per-node flag bitset stored in [`ExprNode::flags`](super::node::ExprNode).
//!
//! Per `dol-rewrite-plan-v2.md` §8.1. Eight bits is enough for the
//! v2-locked flag set; future flags can extend the bitset as long as
//! they fit in the same `u8` field.

/// Bitset packed into [`ExprNode::flags`](super::node::ExprNode).
///
/// Constructed via the typed `new()` / `with_*` builders to keep the
/// raw bit layout an implementation detail.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct NodeFlags(u8);

impl NodeFlags {
    /// Marks a node whose value can be `NULL`. Defaults `false`.
    pub const NULLABLE: u8 = 1 << 0;
    /// Marks a `DISTINCT` aggregate or set operation.
    pub const DISTINCT: u8 = 1 << 1;
    /// Marks a logically-negated form (e.g. the `NOT` half of a
    /// pattern after lowering normalises it).
    pub const NEGATED: u8 = 1 << 2;
    /// Marks an aggregate node — used by the optimizer to decide
    /// whether `GROUP BY` introduction is necessary.
    pub const AGGREGATE: u8 = 1 << 3;

    /// Empty flag set. `const`-friendly so it can be used in
    /// `ExprNode::*` constructors.
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self(0)
    }

    /// Construct from raw bits. Higher-order bits are preserved as-is
    /// — round-tripping a `u8` through `from_bits → to_bits` is a
    /// no-op.
    #[must_use]
    #[inline]
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    /// Underlying `u8` representation.
    #[must_use]
    #[inline]
    pub const fn to_bits(self) -> u8 {
        self.0
    }

    /// `true` when the node may produce `NULL`.
    #[must_use]
    #[inline]
    pub const fn is_nullable(self) -> bool {
        self.0 & Self::NULLABLE != 0
    }

    /// Set the [`NULLABLE`](Self::NULLABLE) flag.
    #[must_use]
    #[inline]
    pub const fn with_nullable(self, on: bool) -> Self {
        Self::set(self.0, Self::NULLABLE, on)
    }

    /// `true` when the node carries `DISTINCT` semantics.
    #[must_use]
    #[inline]
    pub const fn is_distinct(self) -> bool {
        self.0 & Self::DISTINCT != 0
    }

    /// Set the [`DISTINCT`](Self::DISTINCT) flag.
    #[must_use]
    #[inline]
    pub const fn with_distinct(self, on: bool) -> Self {
        Self::set(self.0, Self::DISTINCT, on)
    }

    /// `true` when the node represents a normalised negation of an
    /// otherwise-positive operator.
    #[must_use]
    #[inline]
    pub const fn is_negated(self) -> bool {
        self.0 & Self::NEGATED != 0
    }

    /// Set the [`NEGATED`](Self::NEGATED) flag.
    #[must_use]
    #[inline]
    pub const fn with_negated(self, on: bool) -> Self {
        Self::set(self.0, Self::NEGATED, on)
    }

    /// `true` when the node is an aggregate.
    #[must_use]
    #[inline]
    pub const fn is_aggregate(self) -> bool {
        self.0 & Self::AGGREGATE != 0
    }

    /// Set the [`AGGREGATE`](Self::AGGREGATE) flag.
    #[must_use]
    #[inline]
    pub const fn with_aggregate(self, on: bool) -> Self {
        Self::set(self.0, Self::AGGREGATE, on)
    }

    #[inline]
    const fn set(bits: u8, mask: u8, on: bool) -> Self {
        if on {
            Self(bits | mask)
        } else {
            Self(bits & !mask)
        }
    }
}

impl core::fmt::Debug for NodeFlags {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NodeFlags")
            .field("nullable", &self.is_nullable())
            .field("distinct", &self.is_distinct())
            .field("negated", &self.is_negated())
            .field("aggregate", &self.is_aggregate())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_flags() {
        let f = NodeFlags::new();
        assert!(!f.is_nullable());
        assert!(!f.is_distinct());
        assert!(!f.is_negated());
        assert!(!f.is_aggregate());
        assert_eq!(f.to_bits(), 0);
    }

    #[test]
    fn each_flag_is_independent() {
        let f = NodeFlags::new()
            .with_nullable(true)
            .with_distinct(true)
            .with_negated(true)
            .with_aggregate(true);
        assert!(f.is_nullable());
        assert!(f.is_distinct());
        assert!(f.is_negated());
        assert!(f.is_aggregate());
        assert_eq!(f.to_bits(), 0b1111);
    }

    #[test]
    fn turning_off_clears_only_that_flag() {
        let f = NodeFlags::from_bits(0b1111).with_distinct(false);
        assert!(f.is_nullable());
        assert!(!f.is_distinct());
        assert!(f.is_negated());
        assert!(f.is_aggregate());
        assert_eq!(f.to_bits(), 0b1101);
    }

    #[test]
    fn from_bits_round_trip_preserves_unused_bits() {
        let f = NodeFlags::from_bits(0b1010_1111);
        assert_eq!(f.to_bits(), 0b1010_1111);
    }

    #[test]
    fn debug_format_lists_flags_by_name() {
        let f = NodeFlags::new().with_nullable(true).with_aggregate(true);
        let s = format!("{f:?}");
        assert!(s.contains("nullable: true"));
        assert!(s.contains("distinct: false"));
        assert!(s.contains("aggregate: true"));
    }
}
