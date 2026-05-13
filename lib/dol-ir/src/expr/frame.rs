//! Window / scope frame primitives.
//!
//! Per `dol-rewrite-plan-v2.md` §8.3. These are the parts of the
//! `Context<'a>` family that are independent of [`Expr<'a>`](crate)
//! and so can ship ahead of the tree DSL: pure value types describing
//! a bounded evaluation window over an ordered partition.
//!
//! ## Mental model
//!
//! A window is `[start, end]` boundaries measured in some unit
//! ([`FrameUnit::Rows`] / `Range` / `Groups`). Each [`Boundary`] is
//! either the current row or an [`Extent`] in the past
//! ([`Boundary::Before`]) or the future ([`Boundary::After`]). An
//! `Extent` is either an integer offset or [`Extent::Unbounded`]
//! (the partition edge).
//!
//! Backends translate this to their native window construct:
//! SQL `OVER (ROWS BETWEEN … AND …)`, dataframe `rolling`, IoT stream
//! slice, time-series segment, pipeline partition.
//!
//! ## Why this slice ships first
//!
//! `Context<'a>` and `OrderByExpr<'a>` carry `Expr<'a>` operands and
//! must wait for the tree DSL (M3c-γ). Frame and ordering *primitives*
//! carry no expressions and so can be reviewed in isolation.

// ─── FrameUnit ───────────────────────────────────────────────────────

/// The unit in which window boundaries are measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FrameUnit {
    /// Physical rows: each row counts as one unit.
    Rows,
    /// Logical range based on the ordering expression's value
    /// (e.g. `RANGE BETWEEN INTERVAL '1 day' PRECEDING …`).
    Range,
    /// Distinct values of the ordering expression: each peer group
    /// counts as one unit.
    Groups,
}

// ─── Extent ──────────────────────────────────────────────────────────

/// How far a [`Boundary`] reaches from the current row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Extent {
    /// Reaches to the partition edge. Renders as `UNBOUNDED PRECEDING`
    /// inside [`Boundary::Before`] and `UNBOUNDED FOLLOWING` inside
    /// [`Boundary::After`].
    Unbounded,
    /// Reaches `n` units (rows / range / groups, per [`FrameUnit`])
    /// from the current row.
    Offset(u64),
}

// ─── Boundary ────────────────────────────────────────────────────────

/// One end of a window frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// The current row. Renders as `CURRENT ROW`.
    Current,
    /// In the past, by the given extent. Renders as
    /// `<extent> PRECEDING`.
    Before(Extent),
    /// In the future, by the given extent. Renders as
    /// `<extent> FOLLOWING`.
    After(Extent),
}

impl Boundary {
    /// `UNBOUNDED PRECEDING` — reach the start of the partition.
    #[inline]
    #[must_use]
    pub const fn unbounded_preceding() -> Self {
        Self::Before(Extent::Unbounded)
    }

    /// `UNBOUNDED FOLLOWING` — reach the end of the partition.
    #[inline]
    #[must_use]
    pub const fn unbounded_following() -> Self {
        Self::After(Extent::Unbounded)
    }

    /// `n PRECEDING` — `n` units before the current row.
    #[inline]
    #[must_use]
    pub const fn preceding(n: u64) -> Self {
        Self::Before(Extent::Offset(n))
    }

    /// `n FOLLOWING` — `n` units after the current row.
    #[inline]
    #[must_use]
    pub const fn following(n: u64) -> Self {
        Self::After(Extent::Offset(n))
    }
}

// ─── Frame ───────────────────────────────────────────────────────────

/// A bounded window frame: `unit BETWEEN start AND end`.
///
/// Construct with [`Frame::rows`], [`Frame::range`], or
/// [`Frame::groups`] for clarity at call sites; the literal struct
/// form remains available for callers that need it.
///
/// ## Validity
///
/// This type does not enforce `start ≤ end` at construction time —
/// validation is the lowering layer's job, where it can produce a
/// proper diagnostic with span context. Backends are entitled to
/// reject inverted frames at plan time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Frame {
    /// Unit in which `start` and `end` are measured.
    pub unit: FrameUnit,
    /// Inclusive lower boundary.
    pub start: Boundary,
    /// Inclusive upper boundary.
    pub end: Boundary,
}

impl Frame {
    /// `ROWS BETWEEN start AND end`.
    #[inline]
    #[must_use]
    pub const fn rows(start: Boundary, end: Boundary) -> Self {
        Self {
            unit: FrameUnit::Rows,
            start,
            end,
        }
    }

    /// `RANGE BETWEEN start AND end`.
    #[inline]
    #[must_use]
    pub const fn range(start: Boundary, end: Boundary) -> Self {
        Self {
            unit: FrameUnit::Range,
            start,
            end,
        }
    }

    /// `GROUPS BETWEEN start AND end`.
    #[inline]
    #[must_use]
    pub const fn groups(start: Boundary, end: Boundary) -> Self {
        Self {
            unit: FrameUnit::Groups,
            start,
            end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extent_variants_are_distinct() {
        assert_ne!(Extent::Unbounded, Extent::Offset(0));
        assert_ne!(Extent::Offset(1), Extent::Offset(2));
    }

    #[test]
    fn boundary_helpers_match_explicit_construction() {
        assert_eq!(
            Boundary::unbounded_preceding(),
            Boundary::Before(Extent::Unbounded),
        );
        assert_eq!(
            Boundary::unbounded_following(),
            Boundary::After(Extent::Unbounded),
        );
        assert_eq!(Boundary::preceding(7), Boundary::Before(Extent::Offset(7)));
        assert_eq!(Boundary::following(3), Boundary::After(Extent::Offset(3)));
    }

    #[test]
    fn boundary_current_distinct_from_zero_offsets() {
        // Current is not the same as a 0-offset preceding/following:
        // semantically equivalent for `Rows`, but still distinct
        // values that some backends render differently.
        assert_ne!(Boundary::Current, Boundary::preceding(0));
        assert_ne!(Boundary::Current, Boundary::following(0));
    }

    #[test]
    fn frame_constructors_set_unit() {
        let r = Frame::rows(Boundary::unbounded_preceding(), Boundary::Current);
        assert_eq!(r.unit, FrameUnit::Rows);
        assert_eq!(r.start, Boundary::Before(Extent::Unbounded));
        assert_eq!(r.end, Boundary::Current);

        let g = Frame::groups(Boundary::Current, Boundary::following(2));
        assert_eq!(g.unit, FrameUnit::Groups);

        let rg = Frame::range(Boundary::preceding(5), Boundary::following(5));
        assert_eq!(rg.unit, FrameUnit::Range);
    }

    #[test]
    fn frame_is_copy_and_hash_stable() {
        // POD: must be Copy + Hash + Eq so that ContextBuilder can
        // store it without owning semantics.
        fn assert_copy<T: Copy + Eq + core::hash::Hash>() {}
        assert_copy::<FrameUnit>();
        assert_copy::<Extent>();
        assert_copy::<Boundary>();
        assert_copy::<Frame>();

        // Trip through clone to confirm Copy semantics are correct.
        let f = Frame::rows(
            Boundary::unbounded_preceding(),
            Boundary::unbounded_following(),
        );
        let g = f;
        assert_eq!(f, g);
    }

    #[test]
    fn frame_unit_variants_are_distinct() {
        use core::mem;
        // All three variants exist and are unequal.
        assert_ne!(FrameUnit::Rows, FrameUnit::Range);
        assert_ne!(FrameUnit::Range, FrameUnit::Groups);
        assert_ne!(FrameUnit::Groups, FrameUnit::Rows);
        // Sanity: enum is small (Copy) — matches plan's POD intent.
        assert!(mem::size_of::<FrameUnit>() <= 1);
    }
}
