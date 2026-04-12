//! Ordering types for ORDER BY expressions.

use super::Expr;

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Asc,
    Desc,
}

/// NULLS positioning in ORDER BY.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullsPosition {
    First,
    Last,
}

/// An ORDER BY element using `Expr`.
#[derive(Debug, Clone)]
pub struct OrderByExpr {
    pub expr: Expr,
    pub direction: Direction,
    pub nulls: Option<NullsPosition>,
}
