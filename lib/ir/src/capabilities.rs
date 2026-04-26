//! Backend capability bitset.
//!
//! Each [`Statement`](crate::Statement) declares a set of features it
//! depends on; a backend declares a set of features it provides. `dol-check`
//! refuses any program whose required features aren't a subset of the
//! backend's provided features.
//!
//! Capabilities are an open enumeration encoded as a `u64` bitset. New bits
//! are appended; bits are never reused.

use core::fmt;
use core::ops::{BitAnd, BitOr, BitXor, Not};

/// Bitset of optional backend features.
#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BackendCapabilities(pub u64);

macro_rules! cap_bits {
    ($( $(#[$m:meta])* $name:ident = $bit:expr ;)+) => {
        impl BackendCapabilities {
            $(
                $(#[$m])*
                pub const $name: BackendCapabilities = BackendCapabilities(1u64 << $bit);
            )+

            /// Empty set.
            pub const fn empty() -> Self { BackendCapabilities(0) }

            /// All known capabilities. Useful for tests and "do everything"
            /// in-memory backends.
            pub const ALL: BackendCapabilities = BackendCapabilities($(  (1u64 << $bit) | )+ 0);

            /// `true` if `self` contains every bit in `other`.
            pub const fn contains(self, other: Self) -> bool {
                (self.0 & other.0) == other.0
            }

            /// Bits in `self` that are missing from `other`.
            pub const fn difference(self, other: Self) -> Self {
                BackendCapabilities(self.0 & !other.0)
            }

            /// `true` iff no bits are set.
            pub const fn is_empty(self) -> bool {
                self.0 == 0
            }
        }
    };
}

cap_bits! {
    /// Window functions (`ROW_NUMBER`, `RANK`, `OVER (...)`).
    WINDOW_FUNCTIONS = 0;
    /// Recursive CTEs (`WITH RECURSIVE`).
    RECURSIVE_CTE = 1;
    /// JSON path arrows (`->`, `->>`).
    JSON_ARROWS = 2;
    /// Vector-similarity indexes / operators.
    VECTOR_INDEX = 3;
    /// Geospatial predicates and indexes.
    GEOSPATIAL = 4;
    /// `MERGE` / `UPSERT` semantics.
    MERGE = 5;
    /// Row locking with `FOR UPDATE` / `FOR SHARE`.
    ROW_LOCKING = 6;
    /// `SKIP LOCKED` / `NOWAIT` lock hints.
    LOCK_SKIP_NOWAIT = 7;
    /// Streaming windows + watermarks (from `dol-stream`).
    STREAMING_WINDOWS = 8;
    /// Time-series ops (`time_bucket`, `gap_fill`, `locf`, …).
    TIME_SERIES = 9;
    /// Pipeline / dataflow programs (from `dol-pipeline`).
    PIPELINES = 10;
    /// Object-store statements (`PutObject`, `GetObject`, …).
    OBJECT_STORE = 11;
    /// Filesystem statements (`ReadFile`, `WriteFile`, …).
    FILE_IO = 12;
    /// Row-level security policies.
    POLICIES = 13;
    /// Open extension variant; backend agrees to consult its registry.
    EXTENSIONS = 14;
}

impl BitOr for BackendCapabilities {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
impl BitAnd for BackendCapabilities {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}
impl BitXor for BackendCapabilities {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self {
        Self(self.0 ^ rhs.0)
    }
}
impl Not for BackendCapabilities {
    type Output = Self;
    fn not(self) -> Self {
        Self(!self.0)
    }
}

impl fmt::Debug for BackendCapabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BackendCapabilities(0x{:016x})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_and_difference() {
        let a = BackendCapabilities::WINDOW_FUNCTIONS | BackendCapabilities::JSON_ARROWS;
        let b = BackendCapabilities::WINDOW_FUNCTIONS;
        assert!(a.contains(b));
        assert!(!b.contains(a));
        assert_eq!(a.difference(b), BackendCapabilities::JSON_ARROWS);
    }

    #[test]
    fn empty_and_all_consistent() {
        assert!(BackendCapabilities::empty().is_empty());
        assert!(BackendCapabilities::ALL.contains(BackendCapabilities::EXTENSIONS));
    }
}
