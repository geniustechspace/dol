//! Fluent `Expr` methods and operator overloads.

use std::ops as std_ops;

use super::ast::Expr;
use super::func_def::DolOp;
use super::op_meta::OpDef;
use super::op_registry;
use super::ops::UnaryOp;
use super::order::{Direction, OrderByExpr};
use super::window::WindowBuilder;

impl<'a> Expr<'a> {
    /// Nested field access: `self.field_name`.
    pub fn access(self, name: &str) -> Expr<'a> {
        Expr::FieldAccess {
            base: Box::new(self),
            field: name.to_string(),
        }
    }

    /// Internal helper to construct a non-negated binary op from an `OpDef`.
    fn binop_def(self, op: OpDef, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        Expr::BinaryOp {
            left: Box::new(self),
            op,
            right: Box::new(rhs.into()),
            negated: false,
        }
    }

    /// Negate an expression in-place.
    pub fn negate(self) -> Expr<'a> {
        match self {
            Expr::BinaryOp {
                left,
                op,
                right,
                negated,
            } => Expr::BinaryOp {
                left,
                op,
                right,
                negated: !negated,
            },
            Expr::IsNull { expr, negated } => Expr::IsNull {
                expr,
                negated: !negated,
            },
            Expr::Between {
                expr,
                low,
                high,
                negated,
            } => Expr::Between {
                expr,
                low,
                high,
                negated: !negated,
            },
            Expr::InList {
                expr,
                list,
                negated,
            } => Expr::InList {
                expr,
                list,
                negated: !negated,
            },
            Expr::InSubquery {
                expr,
                subquery,
                negated,
            } => Expr::InSubquery {
                expr,
                subquery,
                negated: !negated,
            },
            Expr::Exists { subquery, negated } => Expr::Exists {
                subquery,
                negated: !negated,
            },
            other => Expr::UnaryOp {
                op: UnaryOp::Not,
                expr: Box::new(other),
            },
        }
    }

    /// `self == rhs`
    pub fn eq(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop_def(op_registry::OpEq::def(), rhs)
    }

    /// `self != rhs`
    pub fn ne(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop_def(op_registry::OpNe::def(), rhs)
    }

    /// `self < rhs`
    pub fn lt(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop_def(op_registry::OpLt::def(), rhs)
    }

    /// `self > rhs`
    pub fn gt(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop_def(op_registry::OpGt::def(), rhs)
    }

    /// `self <= rhs`
    pub fn le(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop_def(op_registry::OpLe::def(), rhs)
    }

    /// `self >= rhs`
    pub fn ge(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop_def(op_registry::OpGe::def(), rhs)
    }

    /// `self IS DISTINCT FROM rhs` (null-safe inequality).
    pub fn is_distinct_from(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop_def(op_registry::OpIsDistinctFrom::def(), rhs)
    }

    /// `self IS NOT DISTINCT FROM rhs` (null-safe equality).
    pub fn is_not_distinct_from(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop_def(op_registry::OpIsNotDistinctFrom::def(), rhs)
    }

    /// `self LIKE rhs`. Chain `.negate()` for `NOT LIKE`.
    pub fn like(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop_def(op_registry::OpLike::def(), rhs)
    }

    /// `self ILIKE rhs`. Chain `.negate()` for `NOT ILIKE`.
    pub fn ilike(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop_def(op_registry::OpIlike::def(), rhs)
    }

    /// `self SIMILAR TO rhs`. Chain `.negate()` for `NOT SIMILAR TO`.
    pub fn similar_to(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop_def(op_registry::OpSimilarTo::def(), rhs)
    }

    /// `self ~ rhs` (POSIX regex match). Chain `.negate()` for `!~`.
    pub fn regex_match(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop_def(op_registry::OpRegexMatch::def(), rhs)
    }

    /// `self ~* rhs` (case-insensitive POSIX regex). Chain `.negate()` for `!~*`.
    pub fn regex_match_insensitive(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop_def(op_registry::OpRegexMatchInsensitive::def(), rhs)
    }

    /// `self IS NULL`. Chain `.negate()` for `IS NOT NULL`.
    pub fn is_null(self) -> Expr<'a> {
        Expr::IsNull {
            expr: Box::new(self),
            negated: false,
        }
    }

    /// `self BETWEEN low AND high`. Chain `.negate()` for `NOT BETWEEN`.
    pub fn between(self, low: impl Into<Expr<'a>>, high: impl Into<Expr<'a>>) -> Expr<'a> {
        Expr::Between {
            expr: Box::new(self),
            low: Box::new(low.into()),
            high: Box::new(high.into()),
            negated: false,
        }
    }

    /// `self IN (list)`. Chain `.negate()` for `NOT IN`.
    pub fn in_list(self, list: Vec<Expr<'a>>) -> Expr<'a> {
        Expr::InList {
            expr: Box::new(self),
            list,
            negated: false,
        }
    }

    /// `self IN (subquery)`. Chain `.negate()` for `NOT IN`.
    pub fn in_subquery(self, subquery: &str) -> Expr<'a> {
        Expr::InSubquery {
            expr: Box::new(self),
            subquery: subquery.to_string(),
            negated: false,
        }
    }

    pub fn cast(self, as_type: crate::types::DataType) -> Expr<'a> {
        Expr::Cast {
            expr: Box::new(self),
            as_type,
        }
    }

    pub fn alias(self, name: &str) -> Expr<'a> {
        Expr::Alias {
            expr: Box::new(self),
            alias: name.to_string(),
        }
    }

    pub fn concat(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop_def(op_registry::OpConcat::def(), rhs)
    }

    /// Create an ascending ORDER BY.
    pub fn asc(self) -> OrderByExpr<'a> {
        OrderByExpr {
            expr: self,
            direction: Direction::Asc,
            nulls: None,
        }
    }

    /// Create a descending ORDER BY.
    pub fn desc(self) -> OrderByExpr<'a> {
        OrderByExpr {
            expr: self,
            direction: Direction::Desc,
            nulls: None,
        }
    }

    pub fn over(self) -> WindowBuilder<'a> {
        WindowBuilder::new(self)
    }
}

impl<'a> std_ops::BitAnd for Expr<'a> {
    type Output = Expr<'a>;

    fn bitand(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp {
            left: Box::new(self),
            op: op_registry::OpAnd::def(),
            right: Box::new(rhs),
            negated: false,
        }
    }
}

impl<'a> std_ops::BitOr for Expr<'a> {
    type Output = Expr<'a>;

    fn bitor(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp {
            left: Box::new(self),
            op: op_registry::OpOr::def(),
            right: Box::new(rhs),
            negated: false,
        }
    }
}

impl<'a> std_ops::Not for Expr<'a> {
    type Output = Expr<'a>;

    fn not(self) -> Expr<'a> {
        Expr::UnaryOp {
            op: UnaryOp::Not,
            expr: Box::new(self),
        }
    }
}

impl<'a> std_ops::Add for Expr<'a> {
    type Output = Expr<'a>;

    fn add(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp {
            left: Box::new(self),
            op: op_registry::OpAdd::def(),
            right: Box::new(rhs),
            negated: false,
        }
    }
}

impl<'a> std_ops::Sub for Expr<'a> {
    type Output = Expr<'a>;

    fn sub(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp {
            left: Box::new(self),
            op: op_registry::OpSub::def(),
            right: Box::new(rhs),
            negated: false,
        }
    }
}

impl<'a> std_ops::Mul for Expr<'a> {
    type Output = Expr<'a>;

    fn mul(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp {
            left: Box::new(self),
            op: op_registry::OpMul::def(),
            right: Box::new(rhs),
            negated: false,
        }
    }
}

impl<'a> std_ops::Div for Expr<'a> {
    type Output = Expr<'a>;

    fn div(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp {
            left: Box::new(self),
            op: op_registry::OpDiv::def(),
            right: Box::new(rhs),
            negated: false,
        }
    }
}

impl<'a> std_ops::Rem for Expr<'a> {
    type Output = Expr<'a>;

    fn rem(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp {
            left: Box::new(self),
            op: op_registry::OpMod::def(),
            right: Box::new(rhs),
            negated: false,
        }
    }
}

impl<'a> std_ops::Neg for Expr<'a> {
    type Output = Expr<'a>;

    fn neg(self) -> Expr<'a> {
        Expr::UnaryOp {
            op: UnaryOp::Neg,
            expr: Box::new(self),
        }
    }
}
