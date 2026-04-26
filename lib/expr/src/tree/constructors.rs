//! Free constructors and coercion traits for the expression DSL.

use alloc::{boxed::Box, vec::Vec};

use super::ast::Expr;
use super::compact_name::CompactName;
use super::literal::Literal;
use super::path::PathExpr;
use super::window::CaseBuilder;

/// Create a leaf field reference: `name`.
///
/// Returns an unanchored [`Expr::Field`] (no namespace base, no in-leaf
/// traversal). Anchor it on a namespace by chaining
/// [`Expr::field()`](Expr::field) on a [`namespace()`] call:
/// `namespace("users").field("email")`.
///
/// String literals are stored as [`CompactName::Static`] (zero alloc).
pub fn field(name: &'static str) -> Expr<'static> {
    Expr::Field {
        base: None,
        name: CompactName::Static(name),
        steps: Vec::new(),
    }
}

/// Create a runtime field reference from a non-static `&str` (allocates).
///
/// Unlike [`field`], this accepts any `&str` at runtime by cloning the string
/// into an owned [`CompactName`]. The returned expression is `Expr<'static>`
/// because no borrowed data is captured.
///
/// Prefer [`field`] for compile-time-known names.
pub fn field_dyn(name: &str) -> Expr<'static> {
    Expr::Field {
        base: None,
        name: CompactName::from_str(name),
        steps: Vec::new(),
    }
}

/// Create a container namespace path: `users`, `schema.users`, etc.
///
/// Multi-segment literals (e.g. `"schema.users"`) are split on `.` and stored
/// as separate segments, so each becomes a [`CompactName::Static`]
/// (zero allocation). A single-segment literal (`"users"`) is also stored
/// `Static`. Use this when an expression *is* a container address (a table,
/// bucket, API endpoint, KV namespace, …). To reference a leaf attribute
/// *inside* a namespace, chain with [`Expr::field()`]:
/// `namespace("users").field("email")`.
pub fn namespace(path: &'static str) -> Expr<'static> {
    let p = if path.contains('.') {
        PathExpr::from_segments(path.split('.').map(CompactName::Static))
    } else {
        PathExpr::one(CompactName::Static(path))
    };
    Expr::Namespace(p)
}

/// Runtime variant of [`namespace`] that accepts any `&str`.
///
/// Multi-segment paths (e.g. `"schema.users"`) are split on `.` and stored
/// as separate segments; each segment is heap-allocated.
pub fn namespace_dyn(path: &str) -> Expr<'static> {
    let p = if path.contains('.') {
        PathExpr::from_segments(path.split('.').map(CompactName::from_str))
    } else {
        PathExpr::from_str(path)
    };
    Expr::Namespace(p)
}

/// Create a qualified leaf reference: `scope.name` (e.g. `users.email`).
///
/// `scope` may itself contain `.` separators (`"schema.users"`); it is split
/// the same way as [`namespace`] so `qualified("schema.users", "email")`
/// produces a two-segment namespace anchor with `email` as the leaf.
/// Equivalent to `namespace(scope).field(name)`.
pub fn qualified(scope: &'static str, name: &'static str) -> Expr<'static> {
    let scope_path = if scope.contains('.') {
        PathExpr::from_segments(scope.split('.').map(CompactName::Static))
    } else {
        PathExpr::one(CompactName::Static(scope))
    };
    Expr::Field {
        base: Some(Box::new(Expr::Namespace(scope_path))),
        name: CompactName::Static(name),
        steps: Vec::new(),
    }
}

/// Create a null literal.
pub fn null<'a>() -> Expr<'a> {
    Expr::Value(Literal::Null)
}

/// Create a string literal.
pub fn string<'a>(v: impl Into<alloc::borrow::Cow<'a, str>>) -> Expr<'a> {
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

/// Convert `&str` to an unanchored [`Expr::Field`] for ergonomic builder use.
///
/// The name is stored as [`CompactName::Owned`] (one heap allocation), since
/// `From<&str>` cannot distinguish a `&'static str` literal from a
/// non-`'static` borrow at the type level. For zero-alloc storage of a
/// compile-time-known name, prefer the [`field`] free function.
impl<'a> From<&str> for Expr<'a> {
    fn from(s: &str) -> Self {
        Expr::Field {
            base: None,
            name: CompactName::from_str(s),
            steps: Vec::new(),
        }
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
