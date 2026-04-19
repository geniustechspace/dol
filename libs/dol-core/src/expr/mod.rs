//! # dol-expr — DOL Expression Engine
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
//! use dol_core::expr::{field, string, int, param};
//!
//! // field reference (DOL primary)
//! let expr = field("email");
//!
//! // typed literal constructors
//! let expr = string("active");
//! let expr = int(42i32);
//!
//! // comparison chain
//! let expr = field("age").gt(int(18i32)) & field("status").eq(string("active"));
//! ```

pub mod func;
mod literal;
mod ops;
pub mod order;
pub mod window;

pub use literal::{Literal, Value, TypeError};
pub use ops::{OpId, Quantifier, UnaryOp};
#[allow(deprecated)]
pub use ops::BinOp;
pub use func::FuncId;
#[allow(deprecated)]
pub use func::FuncName;
pub use order::{Direction, NullsPosition, OrderByExpr};
pub use window::{CaseBuilder, FrameBound, FrameKind, WindowBuilder, WindowFrame};

use std::ops as std_ops;

/// A composable expression node — the core AST type for DOL.
///
/// Every expression is backend-agnostic. Backends (SQL, document, KV) interpret
/// and render expressions according to their own semantics.
///
/// The lifetime `'a` allows zero-copy string literals in the AST via
/// [`Literal<'a>`], which uses `Cow<'a, str>` / `Cow<'a, [u8]>` internally.
///
/// # Examples
///
/// ```rust
/// use dol_core::expr::{field, string, int};
///
/// // Simple comparison: age > 18
/// let expr = field("age").gt(int(18i32));
///
/// // Compound: age > 18 AND status = 'active'
/// let expr = field("age").gt(int(18i32)) & field("status").eq(string("active"));
///
/// // Field access for nested data: profile.address.city
/// let expr = field("profile").access("address").access("city");
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Expr<'a> {
    // ── Identifiers ──
    /// A field reference: `field_name`.
    Identifier(String),
    /// A qualified field reference: `scope.name` (e.g., `users.email`).
    QualifiedIdentifier { scope: String, name: String },
    /// Nested field access: `base.field` (e.g., `profile.address.city`).
    FieldAccess { base: Box<Expr<'a>>, field: String },

    // ── Values ──
    /// A positional bind parameter (`$1`, `?`, `@p1`, `:1`).
    Param,
    /// A literal value from the unified type system.
    Value(Literal<'a>),

    // ── Operations ──
    /// A binary operation: `left op right`, optionally negated.
    ///
    /// The `negated` flag handles `NOT LIKE`, `NOT SIMILAR TO`, etc. without
    /// doubling the operator count.
    BinaryOp {
        left: Box<Expr<'a>>,
        op: OpId,
        right: Box<Expr<'a>>,
        negated: bool,
    },
    /// A unary operation: `op expr`.
    UnaryOp { op: UnaryOp, expr: Box<Expr<'a>> },
    /// A quantified comparison: `expr op ANY(subquery)` / `expr op ALL(subquery)`.
    QuantifiedCmp {
        expr: Box<Expr<'a>>,
        op: OpId,
        quantifier: Quantifier,
        subquery: String,
    },

    // ── Function call ──
    /// A function call: `name(args...)`.
    Func { name: FuncId, args: Vec<Expr<'a>> },

    // ── Type conversion ──
    /// A type cast: `CAST(expr AS type)`.
    Cast { expr: Box<Expr<'a>>, as_type: crate::types::DataType },

    // ── Conditional ──
    /// A CASE expression: `CASE WHEN ... THEN ... ELSE ... END`.
    Case {
        whens: Vec<(Expr<'a>, Expr<'a>)>,
        else_expr: Option<Box<Expr<'a>>>,
    },

    // ── Subquery ──
    /// A subquery: `(SELECT ...)`.
    Subquery(String),

    // ── Set membership ──
    /// `expr [NOT] IN (list)`.
    InList {
        expr: Box<Expr<'a>>,
        list: Vec<Expr<'a>>,
        negated: bool,
    },
    /// `expr [NOT] IN (subquery)`.
    InSubquery {
        expr: Box<Expr<'a>>,
        subquery: String,
        negated: bool,
    },

    // ── Range ──
    /// `expr [NOT] BETWEEN low AND high`.
    Between {
        expr: Box<Expr<'a>>,
        low: Box<Expr<'a>>,
        high: Box<Expr<'a>>,
        negated: bool,
    },

    // ── Existence ──
    /// `[NOT] EXISTS (subquery)`.
    Exists { subquery: String, negated: bool },

    // ── Null check ──
    /// `expr IS [NOT] NULL`.
    IsNull { expr: Box<Expr<'a>>, negated: bool },

    // ── Composites ──
    /// An object literal: `{ key: value, key2: value2 }`.
    ObjectLiteral(Vec<(String, Expr<'a>)>),
    /// An array literal: `[1, 2, 3]`.
    ArrayLiteral(Vec<Expr<'a>>),

    // ── Escape hatch ──
    /// Raw expression string (escape hatch).
    Raw(String),

    // ── Decoration ──
    /// `expr AS alias`.
    Alias { expr: Box<Expr<'a>>, alias: String },
    /// `*` (all fields).
    Star,
    /// `COUNT(*)`.
    CountStar,
    /// A window function: `func OVER (PARTITION BY ... ORDER BY ... frame)`.
    Window {
        func: Box<Expr<'a>>,
        partition_by: Vec<Expr<'a>>,
        order_by: Vec<OrderByExpr<'a>>,
        frame: Option<WindowFrame>,
    },
}

// ---------------------------------------------------------------------------
// Typed constructor functions
// ---------------------------------------------------------------------------

/// Create a field/identifier reference expression (DOL primary constructor).
pub fn field<'a>(name: &str) -> Expr<'a> {
    Expr::Identifier(name.to_string())
}

/// Create a qualified field reference: `scope.name`.
pub fn qualified<'a>(scope: &str, name: &str) -> Expr<'a> {
    Expr::QualifiedIdentifier {
        scope: scope.to_string(),
        name: name.to_string(),
    }
}

/// Create a null literal.
pub fn null<'a>() -> Expr<'a> {
    Expr::Value(Literal::Null)
}

/// Create a string literal.
pub fn string<'a>(v: impl Into<std::borrow::Cow<'a, str>>) -> Expr<'a> {
    Expr::Value(Literal::String(v.into()))
}

/// Create an integer literal. The variant is inferred from the Rust type:
/// `int(42i32)` → `Literal::Int32`, `int(42u64)` → `Literal::UInt64`, etc.
pub fn int<'a>(v: impl IntoIntLiteral) -> Expr<'a> {
    Expr::Value(v.into_int_literal())
}

/// Create a float literal. `float(3.14f32)` → `Literal::Float32`,
/// `float(3.14)` → `Literal::Float64`.
pub fn float<'a>(v: impl IntoFloatLiteral) -> Expr<'a> {
    Expr::Value(v.into_float_literal())
}

/// Create a boolean literal.
pub fn bool_expr<'a>(v: bool) -> Expr<'a> {
    Expr::Value(Literal::Bool(v))
}

/// Create a bind parameter expression.
pub fn param<'a>() -> Expr<'a> {
    Expr::Param
}

/// Create a raw expression string (escape hatch).
pub fn raw_expr<'a>(expr: &str) -> Expr<'a> {
    Expr::Raw(expr.to_string())
}

/// Start building a CASE expression.
pub fn case<'a>() -> CaseBuilder<'a> {
    CaseBuilder::new()
}

/// Create an object literal expression: `{ key: value, ... }`.
pub fn obj<'a>(fields: Vec<(&str, Expr<'a>)>) -> Expr<'a> {
    Expr::ObjectLiteral(
        fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

/// Create an array literal expression: `[elem1, elem2, ...]`.
pub fn arr<'a>(elements: Vec<Expr<'a>>) -> Expr<'a> {
    Expr::ArrayLiteral(elements)
}

// ── Coercion traits for typed constructors ───────────────────────────────

/// Trait for values that can become integer literals.
pub trait IntoIntLiteral {
    fn into_int_literal(self) -> Literal<'static>;
}

macro_rules! impl_into_int {
    ($ty:ty, $variant:ident) => {
        impl IntoIntLiteral for $ty {
            fn into_int_literal(self) -> Literal<'static> { Literal::$variant(self) }
        }
    };
}
impl_into_int!(i8,   Int8);
impl_into_int!(i16,  Int16);
impl_into_int!(i32,  Int32);
impl_into_int!(i64,  Int64);
impl_into_int!(i128, Int128);
impl_into_int!(u8,   UInt8);
impl_into_int!(u16,  UInt16);
impl_into_int!(u32,  UInt32);
impl_into_int!(u64,  UInt64);
impl_into_int!(u128, UInt128);

/// Trait for values that can become float literals.
pub trait IntoFloatLiteral {
    fn into_float_literal(self) -> Literal<'static>;
}

impl IntoFloatLiteral for f32 {
    fn into_float_literal(self) -> Literal<'static> { Literal::Float32(self) }
}
impl IntoFloatLiteral for f64 {
    fn into_float_literal(self) -> Literal<'static> { Literal::Float64(self) }
}

/// Convert `&str` to `Expr::Identifier` for ergonomic builder use.
impl<'a> From<&str> for Expr<'a> {
    fn from(s: &str) -> Self {
        Expr::Identifier(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// Expr method chains
// ---------------------------------------------------------------------------

impl<'a> Expr<'a> {
    // ── Nested field access ──

    /// Nested field access: `self.field_name`.
    pub fn access(self, name: &str) -> Expr<'a> {
        Expr::FieldAccess {
            base: Box::new(self),
            field: name.to_string(),
        }
    }

    // ── Helper ──

    /// Internal helper to construct a non-negated binary op.
    fn binop(self, op: &str, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        Expr::BinaryOp {
            left: Box::new(self),
            op: OpId::new(op),
            right: Box::new(rhs.into()),
            negated: false,
        }
    }

    /// Internal helper to construct a negated binary op.
    fn binop_neg(self, op: &str, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        Expr::BinaryOp {
            left: Box::new(self),
            op: OpId::new(op),
            right: Box::new(rhs.into()),
            negated: true,
        }
    }

    // ── Comparison operators ──

    /// `self == rhs`
    pub fn eq(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(OpId::EQ, rhs)
    }

    /// `self != rhs`
    pub fn ne(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(OpId::NE, rhs)
    }

    /// `self < rhs`
    pub fn lt(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(OpId::LT, rhs)
    }

    /// `self > rhs`
    pub fn gt(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(OpId::GT, rhs)
    }

    /// `self <= rhs`
    pub fn le(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(OpId::LE, rhs)
    }

    /// `self >= rhs`
    pub fn ge(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(OpId::GE, rhs)
    }

    // ── Null-safe comparison ──

    /// `self IS DISTINCT FROM rhs` (null-safe inequality).
    pub fn is_distinct_from(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(OpId::IS_DISTINCT_FROM, rhs)
    }

    /// `self IS NOT DISTINCT FROM rhs` (null-safe equality).
    pub fn is_not_distinct_from(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(OpId::IS_NOT_DISTINCT_FROM, rhs)
    }

    // ── Pattern matching ──

    pub fn like(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> { self.binop(OpId::LIKE, rhs) }
    pub fn not_like(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> { self.binop_neg(OpId::LIKE, rhs) }
    pub fn ilike(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> { self.binop(OpId::ILIKE, rhs) }
    pub fn not_ilike(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> { self.binop_neg(OpId::ILIKE, rhs) }
    pub fn similar_to(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> { self.binop(OpId::SIMILAR_TO, rhs) }
    pub fn not_similar_to(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> { self.binop_neg(OpId::SIMILAR_TO, rhs) }
    pub fn regex_match(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> { self.binop(OpId::REGEX_MATCH, rhs) }
    pub fn not_regex_match(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> { self.binop_neg(OpId::REGEX_MATCH, rhs) }
    pub fn regex_match_insensitive(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> { self.binop(OpId::REGEX_MATCH_INSENSITIVE, rhs) }
    pub fn not_regex_match_insensitive(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> { self.binop_neg(OpId::REGEX_MATCH_INSENSITIVE, rhs) }

    // ── Null checks ──

    pub fn is_null(self) -> Expr<'a> {
        Expr::IsNull { expr: Box::new(self), negated: false }
    }

    pub fn is_not_null(self) -> Expr<'a> {
        Expr::IsNull { expr: Box::new(self), negated: true }
    }

    // ── Range ──

    pub fn between(self, low: impl Into<Expr<'a>>, high: impl Into<Expr<'a>>) -> Expr<'a> {
        Expr::Between {
            expr: Box::new(self),
            low: Box::new(low.into()),
            high: Box::new(high.into()),
            negated: false,
        }
    }

    pub fn not_between(self, low: impl Into<Expr<'a>>, high: impl Into<Expr<'a>>) -> Expr<'a> {
        Expr::Between {
            expr: Box::new(self),
            low: Box::new(low.into()),
            high: Box::new(high.into()),
            negated: true,
        }
    }

    // ── Set membership ──

    pub fn in_list(self, list: Vec<Expr<'a>>) -> Expr<'a> {
        Expr::InList { expr: Box::new(self), list, negated: false }
    }

    pub fn not_in_list(self, list: Vec<Expr<'a>>) -> Expr<'a> {
        Expr::InList { expr: Box::new(self), list, negated: true }
    }

    pub fn in_subquery(self, subquery: &str) -> Expr<'a> {
        Expr::InSubquery { expr: Box::new(self), subquery: subquery.to_string(), negated: false }
    }

    pub fn not_in_subquery(self, subquery: &str) -> Expr<'a> {
        Expr::InSubquery { expr: Box::new(self), subquery: subquery.to_string(), negated: true }
    }

    // ── Type conversion ──

    pub fn cast(self, as_type: crate::types::DataType) -> Expr<'a> {
        Expr::Cast { expr: Box::new(self), as_type }
    }

    // ── Decoration ──

    pub fn alias(self, name: &str) -> Expr<'a> {
        Expr::Alias { expr: Box::new(self), alias: name.to_string() }
    }

    pub fn concat(self, rhs: impl Into<Expr<'a>>) -> Expr<'a> {
        self.binop(OpId::CONCAT, rhs)
    }

    // ── Ordering ──

    pub fn asc(self) -> OrderByExpr<'a> {
        OrderByExpr { expr: self, direction: Direction::Asc, nulls: None }
    }

    pub fn desc(self) -> OrderByExpr<'a> {
        OrderByExpr { expr: self, direction: Direction::Desc, nulls: None }
    }

    pub fn asc_nulls_first(self) -> OrderByExpr<'a> {
        OrderByExpr { expr: self, direction: Direction::Asc, nulls: Some(NullsPosition::First) }
    }

    pub fn asc_nulls_last(self) -> OrderByExpr<'a> {
        OrderByExpr { expr: self, direction: Direction::Asc, nulls: Some(NullsPosition::Last) }
    }

    pub fn desc_nulls_first(self) -> OrderByExpr<'a> {
        OrderByExpr { expr: self, direction: Direction::Desc, nulls: Some(NullsPosition::First) }
    }

    pub fn desc_nulls_last(self) -> OrderByExpr<'a> {
        OrderByExpr { expr: self, direction: Direction::Desc, nulls: Some(NullsPosition::Last) }
    }

    // ── Window ──

    pub fn over(self) -> WindowBuilder<'a> {
        WindowBuilder::new(self)
    }
}

// ---------------------------------------------------------------------------
// Operator overloads
// ---------------------------------------------------------------------------

impl<'a> std_ops::BitAnd for Expr<'a> {
    type Output = Expr<'a>;
    fn bitand(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp { left: Box::new(self), op: OpId::new(OpId::AND), right: Box::new(rhs), negated: false }
    }
}

impl<'a> std_ops::BitOr for Expr<'a> {
    type Output = Expr<'a>;
    fn bitor(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp { left: Box::new(self), op: OpId::new(OpId::OR), right: Box::new(rhs), negated: false }
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
        Expr::BinaryOp { left: Box::new(self), op: OpId::new(OpId::ADD), right: Box::new(rhs), negated: false }
    }
}

impl<'a> std_ops::Sub for Expr<'a> {
    type Output = Expr<'a>;
    fn sub(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp { left: Box::new(self), op: OpId::new(OpId::SUB), right: Box::new(rhs), negated: false }
    }
}

impl<'a> std_ops::Mul for Expr<'a> {
    type Output = Expr<'a>;
    fn mul(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp { left: Box::new(self), op: OpId::new(OpId::MUL), right: Box::new(rhs), negated: false }
    }
}

impl<'a> std_ops::Div for Expr<'a> {
    type Output = Expr<'a>;
    fn div(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp { left: Box::new(self), op: OpId::new(OpId::DIV), right: Box::new(rhs), negated: false }
    }
}

impl<'a> std_ops::Rem for Expr<'a> {
    type Output = Expr<'a>;
    fn rem(self, rhs: Expr<'a>) -> Expr<'a> {
        Expr::BinaryOp { left: Box::new(self), op: OpId::new(OpId::MOD), right: Box::new(rhs), negated: false }
    }
}

impl<'a> std_ops::Neg for Expr<'a> {
    type Output = Expr<'a>;
    fn neg(self) -> Expr<'a> {
        Expr::UnaryOp { op: UnaryOp::Neg, expr: Box::new(self) }
    }
}

#[cfg(test)]
mod tests;
