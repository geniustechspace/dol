//! # dol-core — DOL Expression Engine
//!
//! Composable, backend-agnostic expression AST for DOL.
//!
//! Expressions are the universal building blocks used across all DOL domains
//! (queries, mutations, definitions, etc.). They are backend-agnostic and
//! can be rendered to SQL, document queries, or any other target.
//!
//! # Constructors
//!
//! ```rust
//! use dol_expr::{field, lit, param, raw_expr, case};
//!
//! // field reference (DOL primary)
//! let expr = field("email");
//!
//! // literal value
//! let expr = lit("active");
//!
//! // comparison chain
//! let expr = field("age").gt(lit(18)) & field("status").eq(lit("active"));
//! ```

#![deny(unsafe_code)]

pub mod func;
mod literal;
mod ops;
pub mod order;
pub mod window;

pub use literal::Literal;
pub use ops::{BinOp, Quantifier, TernaryOp, UnaryOp};
pub use order::{Direction, NullsPosition, OrderByExpr};
pub use window::{CaseBuilder, FrameBound, FrameKind, WindowBuilder, WindowFrame};

use std::ops as std_ops;

/// A composable expression node — the core AST type for DOL.
///
/// Every expression is backend-agnostic. Backends (SQL, document, KV) interpret
/// and render expressions according to their own semantics.
///
/// # Examples
///
/// ```rust
/// use dol_expr::{field, lit};
///
/// // Simple comparison: age > 18
/// let expr = field("age").gt(lit(18));
///
/// // Compound: age > 18 AND status = 'active'
/// let expr = field("age").gt(lit(18)) & field("status").eq(lit("active"));
///
/// // Field access for nested data: profile.address.city
/// let expr = field("profile").access("address").access("city");
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Expr {
    // ── Identifiers ──
    /// A field/column reference: `field_name`.
    Identifier(String),
    /// A qualified field reference: `scope.name` (e.g., `users.email`).
    QualifiedIdentifier { scope: String, name: String },
    /// Nested field access: `base.field` (e.g., `profile.address.city`).
    FieldAccess { base: Box<Expr>, field: String },

    // ── Values ──
    /// A positional bind parameter (`$1`, `?`, `@p1`, `:1`).
    Param,
    /// A literal value.
    Literal(Literal),

    // ── Operations ──
    /// A binary operation: `left op right`, optionally negated.
    ///
    /// The `negated` flag handles `NOT LIKE`, `NOT SIMILAR TO`, etc. without
    /// doubling the `BinOp` variant count.
    BinaryOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
        negated: bool,
    },
    /// A unary operation: `op expr`.
    UnaryOp { op: UnaryOp, expr: Box<Expr> },
    /// A ternary operation: `expr op(operand1, operand2)`.
    ///
    /// Canonical example: `expr [NOT] BETWEEN low AND high`.
    TernaryOp {
        expr: Box<Expr>,
        op: TernaryOp,
        first: Box<Expr>,
        second: Box<Expr>,
        negated: bool,
    },
    /// A quantified comparison: `expr op ANY(subquery)` / `expr op ALL(subquery)`.
    QuantifiedCmp {
        expr: Box<Expr>,
        op: BinOp,
        quantifier: Quantifier,
        subquery: String,
    },

    // ── Function call ──
    /// A function call: `name(args...)`.
    Func { name: String, args: Vec<Expr> },

    // ── Type conversion ──
    /// A type cast: `CAST(expr AS type)`.
    Cast { expr: Box<Expr>, as_type: String },

    // ── Conditional ──
    /// A CASE expression: `CASE WHEN ... THEN ... ELSE ... END`.
    Case {
        whens: Vec<(Expr, Expr)>,
        else_expr: Option<Box<Expr>>,
    },

    // ── Subquery ──
    /// A subquery: `(SELECT ...)`.
    Subquery(String),

    // ── Set membership ──
    /// `expr [NOT] IN (list)`.
    InList {
        expr: Box<Expr>,
        list: Vec<Expr>,
        negated: bool,
    },
    /// `expr [NOT] IN (subquery)`.
    InSubquery {
        expr: Box<Expr>,
        subquery: String,
        negated: bool,
    },

    // ── Range ──
    /// `expr [NOT] BETWEEN low AND high`.
    Between {
        expr: Box<Expr>,
        low: Box<Expr>,
        high: Box<Expr>,
        negated: bool,
    },

    // ── Existence ──
    /// `[NOT] EXISTS (subquery)`.
    Exists { subquery: String, negated: bool },

    // ── Null check ──
    /// `expr IS [NOT] NULL`.
    IsNull { expr: Box<Expr>, negated: bool },

    // ── Composites ──
    /// An object literal: `{ key: value, key2: value2 }`.
    ObjectLiteral(Vec<(String, Expr)>),
    /// An array literal: `[1, 2, 3]`.
    ArrayLiteral(Vec<Expr>),

    // ── Escape hatch ──
    /// Raw expression string (escape hatch).
    Raw(String),

    // ── Decoration ──
    /// `expr AS alias`.
    Alias { expr: Box<Expr>, alias: String },
    /// `*` (all fields/columns).
    Star,
    /// `COUNT(*)`.
    CountStar,
    /// A window function: `func OVER (PARTITION BY ... ORDER BY ... frame)`.
    Window {
        func: Box<Expr>,
        partition_by: Vec<Expr>,
        order_by: Vec<OrderByExpr>,
        frame: Option<WindowFrame>,
    },
}

// ---------------------------------------------------------------------------
// Constructor functions
// ---------------------------------------------------------------------------

/// Create a field/identifier reference expression (DOL primary constructor).
pub fn field(name: &str) -> Expr {
    Expr::Identifier(name.to_string())
}

/// Create a qualified field reference: `scope.name`.
pub fn qualified(scope: &str, name: &str) -> Expr {
    Expr::QualifiedIdentifier {
        scope: scope.to_string(),
        name: name.to_string(),
    }
}

/// Create a literal expression from any value that implements `Into<Literal>`.
pub fn lit<T: Into<Literal>>(val: T) -> Expr {
    Expr::Literal(val.into())
}

/// Create a bind parameter expression.
pub fn param() -> Expr {
    Expr::Param
}

/// Create a raw expression string (escape hatch).
pub fn raw_expr(sql: &str) -> Expr {
    Expr::Raw(sql.to_string())
}

/// Start building a CASE expression.
pub fn case() -> CaseBuilder {
    CaseBuilder::new()
}

/// Create an object literal expression: `{ key: value, ... }`.
pub fn obj(fields: Vec<(&str, Expr)>) -> Expr {
    Expr::ObjectLiteral(
        fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

/// Create an array literal expression: `[elem1, elem2, ...]`.
pub fn arr(elements: Vec<Expr>) -> Expr {
    Expr::ArrayLiteral(elements)
}

/// Convert `&str` to `Expr::Identifier` for ergonomic builder use.
impl From<&str> for Expr {
    fn from(s: &str) -> Self {
        Expr::Identifier(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// Expr method chains
// ---------------------------------------------------------------------------

impl Expr {
    // ── Nested field access ──

    /// Nested field access: `self.field_name`.
    ///
    /// ```rust
    /// use dol_expr::field;
    /// let expr = field("profile").access("address").access("city");
    /// ```
    pub fn access(self, name: &str) -> Expr {
        Expr::FieldAccess {
            base: Box::new(self),
            field: name.to_string(),
        }
    }

    // ── Helper ──

    /// Internal helper to construct a non-negated binary op.
    fn binop(self, op: BinOp, rhs: impl Into<Expr>) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op,
            right: Box::new(rhs.into()),
            negated: false,
        }
    }

    /// Internal helper to construct a negated binary op.
    fn binop_neg(self, op: BinOp, rhs: impl Into<Expr>) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op,
            right: Box::new(rhs.into()),
            negated: true,
        }
    }

    // ── Comparison operators ──

    /// `self == rhs`
    pub fn eq(self, rhs: impl Into<Expr>) -> Expr {
        self.binop(BinOp::Eq, rhs)
    }

    /// `self != rhs`
    pub fn ne(self, rhs: impl Into<Expr>) -> Expr {
        self.binop(BinOp::Ne, rhs)
    }

    /// `self < rhs`
    pub fn lt(self, rhs: impl Into<Expr>) -> Expr {
        self.binop(BinOp::Lt, rhs)
    }

    /// `self > rhs`
    pub fn gt(self, rhs: impl Into<Expr>) -> Expr {
        self.binop(BinOp::Gt, rhs)
    }

    /// `self <= rhs`
    pub fn le(self, rhs: impl Into<Expr>) -> Expr {
        self.binop(BinOp::Le, rhs)
    }

    /// `self >= rhs`
    pub fn ge(self, rhs: impl Into<Expr>) -> Expr {
        self.binop(BinOp::Ge, rhs)
    }

    // ── Null-safe comparison ──

    /// `self IS DISTINCT FROM rhs` (null-safe inequality).
    pub fn is_distinct_from(self, rhs: impl Into<Expr>) -> Expr {
        self.binop(BinOp::IsDistinctFrom, rhs)
    }

    /// `self IS NOT DISTINCT FROM rhs` (null-safe equality).
    pub fn is_not_distinct_from(self, rhs: impl Into<Expr>) -> Expr {
        self.binop(BinOp::IsNotDistinctFrom, rhs)
    }

    // ── Pattern matching (support negation via `negated`) ──

    /// `self LIKE rhs`
    pub fn like(self, rhs: impl Into<Expr>) -> Expr {
        self.binop(BinOp::Like, rhs)
    }

    /// `self NOT LIKE rhs`
    pub fn not_like(self, rhs: impl Into<Expr>) -> Expr {
        self.binop_neg(BinOp::Like, rhs)
    }

    /// `self ILIKE rhs` (dialect-aware: falls back to `LOWER(self) LIKE LOWER(rhs)`).
    pub fn ilike(self, rhs: impl Into<Expr>) -> Expr {
        self.binop(BinOp::ILike, rhs)
    }

    /// `self NOT ILIKE rhs`
    pub fn not_ilike(self, rhs: impl Into<Expr>) -> Expr {
        self.binop_neg(BinOp::ILike, rhs)
    }

    /// `self SIMILAR TO rhs` (SQL-standard regex).
    pub fn similar_to(self, rhs: impl Into<Expr>) -> Expr {
        self.binop(BinOp::SimilarTo, rhs)
    }

    /// `self NOT SIMILAR TO rhs`
    pub fn not_similar_to(self, rhs: impl Into<Expr>) -> Expr {
        self.binop_neg(BinOp::SimilarTo, rhs)
    }

    /// `self ~ rhs` (POSIX regex match).
    pub fn regex_match(self, rhs: impl Into<Expr>) -> Expr {
        self.binop(BinOp::RegexMatch, rhs)
    }

    /// `self !~ rhs` (negated POSIX regex match).
    pub fn not_regex_match(self, rhs: impl Into<Expr>) -> Expr {
        self.binop_neg(BinOp::RegexMatch, rhs)
    }

    /// `self ~* rhs` (POSIX regex match, case-insensitive).
    pub fn regex_match_insensitive(self, rhs: impl Into<Expr>) -> Expr {
        self.binop(BinOp::RegexMatchInsensitive, rhs)
    }

    /// `self !~* rhs` (negated case-insensitive regex match).
    pub fn not_regex_match_insensitive(self, rhs: impl Into<Expr>) -> Expr {
        self.binop_neg(BinOp::RegexMatchInsensitive, rhs)
    }

    // ── Null checks ──

    /// `self IS NULL` / `self is null`
    pub fn is_null(self) -> Expr {
        Expr::IsNull {
            expr: Box::new(self),
            negated: false,
        }
    }

    /// `self IS NOT NULL` / `self is not null`
    pub fn is_not_null(self) -> Expr {
        Expr::IsNull {
            expr: Box::new(self),
            negated: true,
        }
    }

    // ── Range ──

    /// `self BETWEEN low AND high`
    pub fn between(self, low: impl Into<Expr>, high: impl Into<Expr>) -> Expr {
        Expr::Between {
            expr: Box::new(self),
            low: Box::new(low.into()),
            high: Box::new(high.into()),
            negated: false,
        }
    }

    /// `self NOT BETWEEN low AND high`
    pub fn not_between(self, low: impl Into<Expr>, high: impl Into<Expr>) -> Expr {
        Expr::Between {
            expr: Box::new(self),
            low: Box::new(low.into()),
            high: Box::new(high.into()),
            negated: true,
        }
    }

    // ── Set membership ──

    /// `self IN (list...)`
    pub fn in_list(self, list: Vec<Expr>) -> Expr {
        Expr::InList {
            expr: Box::new(self),
            list,
            negated: false,
        }
    }

    /// `self NOT IN (list...)`
    pub fn not_in_list(self, list: Vec<Expr>) -> Expr {
        Expr::InList {
            expr: Box::new(self),
            list,
            negated: true,
        }
    }

    /// `self IN (subquery)`
    pub fn in_subquery(self, subquery: &str) -> Expr {
        Expr::InSubquery {
            expr: Box::new(self),
            subquery: subquery.to_string(),
            negated: false,
        }
    }

    /// `self NOT IN (subquery)`
    pub fn not_in_subquery(self, subquery: &str) -> Expr {
        Expr::InSubquery {
            expr: Box::new(self),
            subquery: subquery.to_string(),
            negated: true,
        }
    }

    // ── Type conversion ──

    /// `CAST(self AS type)`
    pub fn cast(self, as_type: &str) -> Expr {
        Expr::Cast {
            expr: Box::new(self),
            as_type: as_type.to_string(),
        }
    }

    // ── Decoration ──

    /// `self AS alias`
    pub fn alias(self, name: &str) -> Expr {
        Expr::Alias {
            expr: Box::new(self),
            alias: name.to_string(),
        }
    }

    /// String concatenation (dialect-aware: `||` vs `CONCAT()` vs `+`).
    pub fn concat(self, rhs: impl Into<Expr>) -> Expr {
        self.binop(BinOp::Concat, rhs)
    }

    // ── Ordering ──

    /// `self ASC`
    pub fn asc(self) -> OrderByExpr {
        OrderByExpr {
            expr: self,
            direction: Direction::Asc,
            nulls: None,
        }
    }

    /// `self DESC`
    pub fn desc(self) -> OrderByExpr {
        OrderByExpr {
            expr: self,
            direction: Direction::Desc,
            nulls: None,
        }
    }

    /// `self ASC NULLS FIRST`
    pub fn asc_nulls_first(self) -> OrderByExpr {
        OrderByExpr {
            expr: self,
            direction: Direction::Asc,
            nulls: Some(NullsPosition::First),
        }
    }

    /// `self ASC NULLS LAST`
    pub fn asc_nulls_last(self) -> OrderByExpr {
        OrderByExpr {
            expr: self,
            direction: Direction::Asc,
            nulls: Some(NullsPosition::Last),
        }
    }

    /// `self DESC NULLS FIRST`
    pub fn desc_nulls_first(self) -> OrderByExpr {
        OrderByExpr {
            expr: self,
            direction: Direction::Desc,
            nulls: Some(NullsPosition::First),
        }
    }

    /// `self DESC NULLS LAST`
    pub fn desc_nulls_last(self) -> OrderByExpr {
        OrderByExpr {
            expr: self,
            direction: Direction::Desc,
            nulls: Some(NullsPosition::Last),
        }
    }

    // ── Window ──

    /// Start building a window function: `self OVER (...)`.
    pub fn over(self) -> WindowBuilder {
        WindowBuilder::new(self)
    }
}

// ---------------------------------------------------------------------------
// Operator overloads
// ---------------------------------------------------------------------------

/// `expr_a & expr_b` → `expr_a AND expr_b`
impl std_ops::BitAnd for Expr {
    type Output = Expr;
    fn bitand(self, rhs: Expr) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::And,
            right: Box::new(rhs),
            negated: false,
        }
    }
}

/// `expr_a | expr_b` → `expr_a OR expr_b`
impl std_ops::BitOr for Expr {
    type Output = Expr;
    fn bitor(self, rhs: Expr) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Or,
            right: Box::new(rhs),
            negated: false,
        }
    }
}

/// `!expr` → `NOT expr`
impl std_ops::Not for Expr {
    type Output = Expr;
    fn not(self) -> Expr {
        Expr::UnaryOp {
            op: UnaryOp::Not,
            expr: Box::new(self),
        }
    }
}

/// `expr_a + expr_b`
impl std_ops::Add for Expr {
    type Output = Expr;
    fn add(self, rhs: Expr) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Add,
            right: Box::new(rhs),
            negated: false,
        }
    }
}

/// `expr_a - expr_b`
impl std_ops::Sub for Expr {
    type Output = Expr;
    fn sub(self, rhs: Expr) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Sub,
            right: Box::new(rhs),
            negated: false,
        }
    }
}

/// `expr_a * expr_b`
impl std_ops::Mul for Expr {
    type Output = Expr;
    fn mul(self, rhs: Expr) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Mul,
            right: Box::new(rhs),
            negated: false,
        }
    }
}

/// `expr_a / expr_b`
impl std_ops::Div for Expr {
    type Output = Expr;
    fn div(self, rhs: Expr) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Div,
            right: Box::new(rhs),
            negated: false,
        }
    }
}

/// `expr_a % expr_b`
impl std_ops::Rem for Expr {
    type Output = Expr;
    fn rem(self, rhs: Expr) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Mod,
            right: Box::new(rhs),
            negated: false,
        }
    }
}

/// `-expr`
impl std_ops::Neg for Expr {
    type Output = Expr;
    fn neg(self) -> Expr {
        Expr::UnaryOp {
            op: UnaryOp::Neg,
            expr: Box::new(self),
        }
    }
}

#[cfg(test)]
mod tests;
