//! Window function builders and frame specifications.

use alloc::{boxed::Box, vec::Vec};

use super::Expr;
use super::order::OrderByExpr;

/// Window frame bound for ROWS/RANGE BETWEEN.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum FrameBound {
    UnboundedPreceding,
    Preceding(u32),
    CurrentRow,
    Following(u32),
    UnboundedFollowing,
}

/// A window frame specification: `ROWS/RANGE BETWEEN start AND end`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WindowFrame {
    pub kind: FrameKind,
    pub start: FrameBound,
    pub end: Option<FrameBound>,
}

/// Frame kind for window specifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum FrameKind {
    Rows,
    Range,
}

/// Builder for window function specifications: `func OVER (...)`.
///
/// **Note:** Builder helper types (`WindowBuilder`, [`CaseBuilder`]) are
/// transient construction state and intentionally **not** part of the
/// `serde` surface. Persist or transmit the produced [`Expr`] (which does
/// implement `Serialize`/`Deserialize` under the `serde` feature) instead.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowBuilder<'a> {
    func: Expr<'a>,
    partition_by: Vec<Expr<'a>>,
    order_by: Vec<OrderByExpr<'a>>,
    frame: Option<WindowFrame>,
}

impl<'a> WindowBuilder<'a> {
    pub fn new(func: Expr<'a>) -> Self {
        Self {
            func,
            partition_by: Vec::new(),
            order_by: Vec::new(),
            frame: None,
        }
    }

    /// `PARTITION BY exprs...`
    pub fn partition_by(mut self, exprs: impl IntoIterator<Item = Expr<'a>>) -> Self {
        self.partition_by = exprs.into_iter().collect();
        self
    }

    /// `ORDER BY order_exprs...`
    pub fn order_by(mut self, exprs: impl IntoIterator<Item = OrderByExpr<'a>>) -> Self {
        self.order_by = exprs.into_iter().collect();
        self
    }

    /// `ROWS BETWEEN start AND end`
    pub fn rows_between(mut self, start: FrameBound, end: FrameBound) -> Self {
        self.frame = Some(WindowFrame {
            kind: FrameKind::Rows,
            start,
            end: Some(end),
        });
        self
    }

    /// `RANGE BETWEEN start AND end`
    pub fn range_between(mut self, start: FrameBound, end: FrameBound) -> Self {
        self.frame = Some(WindowFrame {
            kind: FrameKind::Range,
            start,
            end: Some(end),
        });
        self
    }

    /// Build the final window expression.
    pub fn build(self) -> Expr<'a> {
        Expr::Window {
            func: Box::new(self.func),
            partition_by: self.partition_by,
            order_by: self.order_by,
            frame: self.frame,
        }
    }
}

/// Builder for CASE WHEN ... THEN ... ELSE ... END expressions.
///
/// **Note:** Builder helper types ([`WindowBuilder`], `CaseBuilder`) are
/// transient construction state and intentionally **not** part of the
/// `serde` surface. Persist or transmit the produced [`Expr`] (which does
/// implement `Serialize`/`Deserialize` under the `serde` feature) instead.
#[derive(Debug, Clone, PartialEq)]
pub struct CaseBuilder<'a> {
    whens: Vec<(Expr<'a>, Expr<'a>)>,
    else_expr: Option<Expr<'a>>,
}

impl<'a> Default for CaseBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> CaseBuilder<'a> {
    pub fn new() -> Self {
        Self {
            whens: Vec::new(),
            else_expr: None,
        }
    }

    /// Add a `WHEN condition THEN result` clause.
    pub fn when(mut self, condition: impl Into<Expr<'a>>, result: impl Into<Expr<'a>>) -> Self {
        self.whens.push((condition.into(), result.into()));
        self
    }

    /// Set the `ELSE result` clause.
    pub fn else_(mut self, result: impl Into<Expr<'a>>) -> Self {
        self.else_expr = Some(result.into());
        self
    }

    /// Build the final CASE expression.
    pub fn end(self) -> Expr<'a> {
        Expr::Case {
            whens: self.whens,
            else_expr: self.else_expr.map(Box::new),
        }
    }
}
