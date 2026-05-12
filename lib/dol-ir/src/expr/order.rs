//! Sort direction and null ordering for window / scope ordering.
//!
//! Per `dol-rewrite-plan-v2.md` §8.3. These primitives ride along
//! inside an `OrderByExpr<'a>` (which lands with the `Expr<'a>` enum
//! in M3c-γ); shipping them now lets the frame primitives in
//! [`crate::expr::frame`] form a complete ordering vocabulary that
//! the lowering layer and backends can consume.

// ─── SortDirection ───────────────────────────────────────────────────

/// Direction of a sort.
///
/// Default is [`SortDirection::Asc`] for parity with SQL `ORDER BY`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SortDirection {
    /// Ascending (`ASC`). Default.
    #[default]
    Asc,
    /// Descending (`DESC`).
    Desc,
}

impl SortDirection {
    /// Returns `true` for ascending, `false` for descending. Useful
    /// for backends that only carry a boolean.
    #[inline]
    #[must_use]
    pub const fn is_ascending(self) -> bool {
        matches!(self, Self::Asc)
    }

    /// Returns the reverse direction.
    #[inline]
    #[must_use]
    pub const fn reversed(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }
}

// ─── NullsOrder ──────────────────────────────────────────────────────

/// Where NULLs sort relative to non-NULL values.
///
/// [`NullsOrder::Default`] defers to the backend's native rule
/// (e.g. PostgreSQL: NULLs last for `ASC`, NULLs first for `DESC`;
/// SQL Server: the inverse). Specify [`NullsOrder::First`] or
/// [`NullsOrder::Last`] explicitly to override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NullsOrder {
    /// `NULLS FIRST` — NULLs sort before non-NULL values.
    First,
    /// `NULLS LAST` — NULLs sort after non-NULL values.
    Last,
    /// Backend-default rule. Default.
    #[default]
    Default,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_direction_default_is_asc() {
        assert_eq!(SortDirection::default(), SortDirection::Asc);
    }

    #[test]
    fn sort_direction_is_ascending() {
        assert!(SortDirection::Asc.is_ascending());
        assert!(!SortDirection::Desc.is_ascending());
    }

    #[test]
    fn sort_direction_reversed_is_involutive() {
        for d in [SortDirection::Asc, SortDirection::Desc] {
            assert_eq!(d.reversed().reversed(), d);
        }
        assert_eq!(SortDirection::Asc.reversed(), SortDirection::Desc);
        assert_eq!(SortDirection::Desc.reversed(), SortDirection::Asc);
    }

    #[test]
    fn nulls_order_default_is_default_variant() {
        // The backend-default rule is the safest fall-through —
        // matching the variant chosen by `#[default]`.
        assert_eq!(NullsOrder::default(), NullsOrder::Default);
    }

    #[test]
    fn nulls_order_variants_are_distinct() {
        assert_ne!(NullsOrder::First, NullsOrder::Last);
        assert_ne!(NullsOrder::First, NullsOrder::Default);
        assert_ne!(NullsOrder::Last, NullsOrder::Default);
    }

    #[test]
    fn order_primitives_are_copy() {
        fn assert_copy<T: Copy + Eq + core::hash::Hash + Default>() {}
        assert_copy::<SortDirection>();
        assert_copy::<NullsOrder>();
    }
}
