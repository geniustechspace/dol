//! Free constructors and coercion traits for the expression DSL.

use super::ast::Expr;
use super::compact_name::CompactName;
use super::literal::Literal;
use super::path::PathExpr;
use super::window::CaseBuilder;

/// Create a field reference: `field_name`.
///
/// The name is a string literal → stored as `CompactName::Static` (zero alloc).
pub fn field(name: &'static str) -> Expr<'static> {
    Expr::Ref(PathExpr::one(name))
}

/// Create a runtime field reference from a non-static `&str` (allocates).
///
/// Unlike [`field`], this accepts any `&str` at runtime by cloning the string
/// into an owned `CompactName`. The returned expression is `Expr<'static>`
/// because no borrowed data is captured.
///
/// Prefer [`field`] for compile-time-known names.
pub fn field_dyn(name: &str) -> Expr<'static> {
    Expr::Ref(PathExpr::from_str(name))
}

/// Create a qualified reference: `scope.name` (e.g. `"users"`, `"email"`).
///
/// Both segments are string literals → zero alloc.
pub fn qualified(scope: &'static str, name: &'static str) -> Expr<'static> {
    Expr::Ref(PathExpr::from_segments([scope, name]))
}

/// Create a null literal.
pub fn null<'a>() -> Expr<'a> {
    Expr::Value(Literal::Null)
}

/// Create a string literal.
pub fn string<'a>(v: impl Into<std::borrow::Cow<'a, str>>) -> Expr<'a> {
    Expr::Value(Literal::String(v.into()))
}

/// Create an integer literal. The variant is inferred from the Rust type.
pub fn int<'a>(v: impl IntoIntLiteral) -> Expr<'a> {
    Expr::Value(v.into_int_literal())
}

/// Create a float literal.
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

/// Start building a `CASE` expression.
pub fn case<'a>() -> CaseBuilder<'a> {
    CaseBuilder::new()
}

/// Create an object literal: `{ key: value, ... }`.
///
/// Keys are `&str` and stored as `CompactName::Owned`.
pub fn obj<'a>(fields: Vec<(&str, Expr<'a>)>) -> Expr<'a> {
    Expr::Object(
        fields
            .into_iter()
            .map(|(k, v)| (CompactName::from_str(k), v))
            .collect(),
    )
}

/// Create an array literal: `[elem1, elem2, ...]`.
pub fn arr<'a>(elements: Vec<Expr<'a>>) -> Expr<'a> {
    Expr::Array(elements)
}

/// Convert `&str` to `Expr::Ref` for ergonomic builder use.
///
/// String literals use `CompactName::Static` (zero alloc). Non-static `&str`
/// creates an `Owned` variant.
impl<'a> From<&str> for Expr<'a> {
    fn from(s: &str) -> Self {
        Expr::Ref(PathExpr::from_str(s))
    }
}

// ── Integer coercion ─────────────────────────────────────────────────────────

/// Trait for values that can become integer literals.
pub trait IntoIntLiteral {
    fn into_int_literal(self) -> Literal<'static>;
}

macro_rules! impl_into_int {
    ($ty:ty, $variant:ident) => {
        impl IntoIntLiteral for $ty {
            fn into_int_literal(self) -> Literal<'static> {
                Literal::$variant(self)
            }
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

// ── Float coercion ───────────────────────────────────────────────────────────

/// Trait for values that can become float literals.
pub trait IntoFloatLiteral {
    fn into_float_literal(self) -> Literal<'static>;
}

impl IntoFloatLiteral for f32 {
    fn into_float_literal(self) -> Literal<'static> {
        Literal::Float32(self)
    }
}

impl IntoFloatLiteral for f64 {
    fn into_float_literal(self) -> Literal<'static> {
        Literal::Float64(self)
    }
}
