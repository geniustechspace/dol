//! Free constructors and coercion traits for the expression DSL.

use super::ast::Expr;
use super::literal::Literal;
use super::window::CaseBuilder;

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

impl_into_int!(i8, Int8);
impl_into_int!(i16, Int16);
impl_into_int!(i32, Int32);
impl_into_int!(i64, Int64);
impl_into_int!(i128, Int128);
impl_into_int!(u8, UInt8);
impl_into_int!(u16, UInt16);
impl_into_int!(u32, UInt32);
impl_into_int!(u64, UInt64);
impl_into_int!(u128, UInt128);

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

/// Convert `&str` to `Expr::Identifier` for ergonomic builder use.
impl<'a> From<&str> for Expr<'a> {
    fn from(s: &str) -> Self {
        Expr::Identifier(s.to_string())
    }
}
