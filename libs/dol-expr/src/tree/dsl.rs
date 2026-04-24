//! Fluent `Expr` methods and operator overloads.

use std::ops as std_ops;

use super::ast::Expr;
use super::compact_name::CompactName;
use super::func_def::DolOp;
use super::op_meta::OpDef;
use super::op_registry;
use super::ops::UnaryOp;
use super::order::{Direction, OrderByExpr};
use super::path::PathExpr;
use super::window::WindowBuilder;

impl<'a> Expr<'a> {
    /// Access a sub-field of this expression: `self.field_name`.
    ///
    /// Consecutive `.get()` calls extend the path rather than nesting boxes:
    /// `field("profile").get("address").get("city")` →
    /// `Access { base: Ref(["profile"]), path: ["address", "city"] }`.
    pub fn get(self, name: &str) -> Expr<'a> {
        match self {
            // Extend an existing Access node's path — avoids an extra Box.
            Expr::Access { base, mut path } => {
                path.segments.push(CompactName::from_str(name));
                Expr::Access { base, path }
            }
            // Any other base (including Ref) creates a new Access node.
            other => Expr::Access {
                base: Box::new(other),
                path: PathExpr::from_str(name),
            },
        }
    }

    /// Internal helper: build a non-negated binary op.
    fn binop(self, op: OpDef, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        Expr::BinaryOp {
            left:  Box::new(self),
            op,
            right: Box::new(rhs.into()),
        }
    }

    /// Negate this expression.
    ///
    /// - `IS NULL` ↔ `IS NOT NULL` (toggled without wrapping).
    /// - `NOT (NOT x)` → `x` (double-negation eliminated).
    /// - All other expressions are wrapped in `UnaryOp::Not`.
    pub fn negate(self) -> Expr<'a> {
        match self {
            Expr::UnaryOp { op: UnaryOp::IsNull,    expr } =>
                Expr::UnaryOp { op: UnaryOp::IsNotNull, expr },
            Expr::UnaryOp { op: UnaryOp::IsNotNull, expr } =>
                Expr::UnaryOp { op: UnaryOp::IsNull, expr },
            Expr::UnaryOp { op: UnaryOp::Not, expr } => *expr,
            other => Expr::UnaryOp { op: UnaryOp::Not, expr: Box::new(other) },
        }
    }

    // ── Comparisons ──────────────────────────────────────────────────────────

    /// `self = rhs`
    pub fn eq(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(op_registry::OpEq::def(), rhs)
    }

    /// `self != rhs`
    pub fn ne(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(op_registry::OpNe::def(), rhs)
    }

    /// `self < rhs`
    pub fn lt(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(op_registry::OpLt::def(), rhs)
    }

    /// `self > rhs`
    pub fn gt(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(op_registry::OpGt::def(), rhs)
    }

    /// `self <= rhs`
    pub fn le(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(op_registry::OpLe::def(), rhs)
    }

    /// `self >= rhs`
    pub fn ge(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(op_registry::OpGe::def(), rhs)
    }

    /// `self IS DISTINCT FROM rhs` (null-safe inequality).
    pub fn is_distinct_from(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(op_registry::OpIsDistinctFrom::def(), rhs)
    }

    /// `self IS NOT DISTINCT FROM rhs` (null-safe equality).
    pub fn is_not_distinct_from(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(op_registry::OpIsNotDistinctFrom::def(), rhs)
    }

    // ── Pattern matching ─────────────────────────────────────────────────────

    /// `self LIKE rhs`. Chain `.negate()` for `NOT LIKE`.
    pub fn like(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(op_registry::OpLike::def(), rhs)
    }

    /// `self ILIKE rhs`. Chain `.negate()` for `NOT ILIKE`.
    pub fn ilike(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(op_registry::OpIlike::def(), rhs)
    }

    /// `self SIMILAR TO rhs`. Chain `.negate()` for `NOT SIMILAR TO`.
    pub fn similar_to(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(op_registry::OpSimilarTo::def(), rhs)
    }

    /// `self ~ rhs` (POSIX regex match). Chain `.negate()` for `!~`.
    pub fn regex_match(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(op_registry::OpRegexMatch::def(), rhs)
    }

    /// `self ~* rhs` (case-insensitive POSIX regex). Chain `.negate()` for `!~*`.
    pub fn regex_match_insensitive(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(op_registry::OpRegexMatchInsensitive::def(), rhs)
    }

    // ── Null checks ──────────────────────────────────────────────────────────

    /// `self IS NULL`. Chain `.negate()` for `IS NOT NULL`.
    pub fn is_null(self) -> Expr<'a> {
        Expr::UnaryOp {
            op:   UnaryOp::IsNull,
            expr: Box::new(self),
        }
    }

    // ── Range / set ──────────────────────────────────────────────────────────

    /// `self BETWEEN low AND high`. Chain `.negate()` for `NOT BETWEEN`.
    pub fn between(
        self,
        low:  impl Into<Expr<'a>>,
        high: impl Into<Expr<'a>>,
    ) -> Expr<'a> {
        Expr::Between {
            expr: Box::new(self),
            low:  Box::new(low.into()),
            high: Box::new(high.into()),
        }
    }

    /// `self IN (list)`. Chain `.negate()` for `NOT IN`.
    pub fn in_list(self, list: Vec<Expr<'a>>) -> Expr<'a> {
        Expr::InList {
            expr: Box::new(self),
            list,
        }
    }

    // ── Type / decoration ────────────────────────────────────────────────────

    /// `CAST(self AS as_type)`.
    pub fn cast(self, as_type: crate::types::DataType) -> Expr<'a> {
        Expr::Cast {
            expr:    Box::new(self),
            as_type,
        }
    }

    /// `self AS alias`.
    pub fn alias(self, name: &str) -> Expr<'a> {
        Expr::Alias {
            expr:  Box::new(self),
            alias: CompactName::from_str(name),
        }
    }

    /// String concatenation: `self || rhs`.
    pub fn concat(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(op_registry::OpConcat::def(), rhs)
    }

    // ── Ordering ─────────────────────────────────────────────────────────────

    /// Ascending `ORDER BY`.
    pub fn asc(self) -> OrderByExpr<'a> {
        OrderByExpr {
            expr:      self,
            direction: Direction::Asc,
            nulls:     None,
        }
    }

    /// Descending `ORDER BY`.
    pub fn desc(self) -> OrderByExpr<'a> {
        OrderByExpr {
            expr:      self,
            direction: Direction::Desc,
            nulls:     None,
        }
    }

    /// Start building a window function `OVER` clause.
    pub fn over(self) -> WindowBuilder<'a> {
        WindowBuilder::new(self)
    }
}

// ── Operator overloads ───────────────────────────────────────────────────────

impl<'a> std_ops::BitAnd for Expr<'a> {
    type Output = Expr<'a>;
    fn bitand(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp {
            left:  Box::new(self),
            op:    op_registry::OpAnd::def(),
            right: Box::new(rhs),
        }
    }
}

impl<'a> std_ops::BitOr for Expr<'a> {
    type Output = Expr<'a>;
    fn bitor(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp {
            left:  Box::new(self),
            op:    op_registry::OpOr::def(),
            right: Box::new(rhs),
        }
    }
}

impl<'a> std_ops::Not for Expr<'a> {
    type Output = Expr<'a>;
    fn not(self) -> Expr<'a> {
        Expr::UnaryOp { op: UnaryOp::Not, expr: Box::new(self) }
    }
}

impl<'a> std_ops::Add for Expr<'a> {
    type Output = Expr<'a>;
    fn add(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp { left: Box::new(self), op: op_registry::OpAdd::def(), right: Box::new(rhs) }
    }
}

impl<'a> std_ops::Sub for Expr<'a> {
    type Output = Expr<'a>;
    fn sub(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp { left: Box::new(self), op: op_registry::OpSub::def(), right: Box::new(rhs) }
    }
}

impl<'a> std_ops::Mul for Expr<'a> {
    type Output = Expr<'a>;
    fn mul(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp { left: Box::new(self), op: op_registry::OpMul::def(), right: Box::new(rhs) }
    }
}

impl<'a> std_ops::Div for Expr<'a> {
    type Output = Expr<'a>;
    fn div(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp { left: Box::new(self), op: op_registry::OpDiv::def(), right: Box::new(rhs) }
    }
}

impl<'a> std_ops::Rem for Expr<'a> {
    type Output = Expr<'a>;
    fn rem(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp { left: Box::new(self), op: op_registry::OpMod::def(), right: Box::new(rhs) }
    }
}

impl<'a> std_ops::Neg for Expr<'a> {
    type Output = Expr<'a>;
    fn neg(self) -> Expr<'a> {
        Expr::UnaryOp { op: UnaryOp::Neg, expr: Box::new(self) }
    }
}
