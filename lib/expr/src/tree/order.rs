//! Ordering types for ORDER BY expressions.

use super::Expr;

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Direction {
    Asc,
    Desc,
}

/// NULLS positioning in ORDER BY.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum NullsPosition {
    First,
    Last,
}

/// An ORDER BY element using `Expr`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct OrderByExpr<'a> {
    pub expr: Expr<'a>,
    pub direction: Direction,
    pub nulls: Option<NullsPosition>,
}

impl<'a> OrderByExpr<'a> {
    /// Set NULLS FIRST positioning.
    pub fn nulls_first(mut self) -> Self {
        self.nulls = Some(NullsPosition::First);
        self
    }

    /// Set NULLS LAST positioning.
    pub fn nulls_last(mut self) -> Self {
        self.nulls = Some(NullsPosition::Last);
        self
    }
}
