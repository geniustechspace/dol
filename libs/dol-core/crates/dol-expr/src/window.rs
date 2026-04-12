//! Window function builders and frame specifications.

use super::Expr;
use super::order::OrderByExpr;

/// Window frame bound for ROWS/RANGE BETWEEN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameBound {
    UnboundedPreceding,
    Preceding(u32),
    CurrentRow,
    Following(u32),
    UnboundedFollowing,
}

/// A window frame specification: `ROWS/RANGE BETWEEN start AND end`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowFrame {
    pub kind: FrameKind,
    pub start: FrameBound,
    pub end: Option<FrameBound>,
}

/// Frame kind for window specifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    Rows,
    Range,
}

/// Builder for window function specifications: `func OVER (...)`.
#[derive(Debug, Clone)]
pub struct WindowBuilder {
    func: Expr,
    partition_by: Vec<Expr>,
    order_by: Vec<OrderByExpr>,
    frame: Option<WindowFrame>,
}

impl WindowBuilder {
    pub fn new(func: Expr) -> Self {
        Self {
            func,
            partition_by: Vec::new(),
            order_by: Vec::new(),
            frame: None,
        }
    }

    /// `PARTITION BY exprs...`
    pub fn partition_by(mut self, exprs: impl IntoIterator<Item = Expr>) -> Self {
        self.partition_by = exprs.into_iter().collect();
        self
    }

    /// `ORDER BY order_exprs...`
    pub fn order_by(mut self, exprs: impl IntoIterator<Item = OrderByExpr>) -> Self {
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
    pub fn build(self) -> Expr {
        Expr::Window {
            func: Box::new(self.func),
            partition_by: self.partition_by,
            order_by: self.order_by,
            frame: self.frame,
        }
    }
}

/// Builder for CASE WHEN ... THEN ... ELSE ... END expressions.
#[derive(Debug, Clone)]
pub struct CaseBuilder {
    whens: Vec<(Expr, Expr)>,
    else_expr: Option<Expr>,
}

impl Default for CaseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CaseBuilder {
    pub fn new() -> Self {
        Self {
            whens: Vec::new(),
            else_expr: None,
        }
    }

    /// Add a `WHEN condition THEN result` clause.
    pub fn when(mut self, condition: impl Into<Expr>, result: impl Into<Expr>) -> Self {
        self.whens.push((condition.into(), result.into()));
        self
    }

    /// Set the `ELSE result` clause.
    pub fn else_(mut self, result: impl Into<Expr>) -> Self {
        self.else_expr = Some(result.into());
        self
    }

    /// Build the final CASE expression.
    pub fn end(self) -> Expr {
        Expr::Case {
            whens: self.whens,
            else_expr: self.else_expr.map(Box::new),
        }
    }
}
