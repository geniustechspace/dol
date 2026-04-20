//! Expression AST definitions.

use super::func_meta::FuncDef;
use super::literal::Literal;
use super::op_meta::OpDef;
use super::ops::{Quantifier, UnaryOp};
use super::order::OrderByExpr;
use super::window::WindowFrame;

/// A composable expression node, the core AST type for DOL.
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
/// let expr = field("age").gt(int(18i32));
/// let expr = field("age").gt(int(18i32)) & field("status").eq(string("active"));
/// let expr = field("profile").get("address").get("city");
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Expr<'a> {
    // Identifiers
    /// A field reference: `field_name`.
    Identifier(String),
    /// A qualified field reference: `scope.name` (e.g., `users.email`).
    QualifiedIdentifier { scope: String, name: String },
    /// Nested field access: `base.field` (e.g., `profile.address.city`).
    FieldAccess { base: Box<Expr<'a>>, field: String },

    // Values
    /// A positional bind parameter (`$1`, `?`, `@p1`, `:1`).
    Param,
    /// A literal value from the unified type system.
    Value(Literal<'a>),

    // Operations
    /// A binary operation: `left op right`, optionally negated.
    ///
    /// The `negated` flag handles `NOT LIKE`, `NOT SIMILAR TO`, etc. without
    /// doubling the operator count.
    BinaryOp {
        left: Box<Expr<'a>>,
        op: OpDef,
        right: Box<Expr<'a>>,
        negated: bool,
    },
    /// A unary operation: `op expr`.
    UnaryOp { op: UnaryOp, expr: Box<Expr<'a>> },
    /// A quantified comparison: `expr op ANY(subquery)` / `expr op ALL(subquery)`.
    QuantifiedCmp {
        expr: Box<Expr<'a>>,
        op: OpDef,
        quantifier: Quantifier,
        subquery: String,
    },

    // Function call
    /// A function call: `name(args...)`.
    Func { name: FuncDef, args: Vec<Expr<'a>> },

    // Type conversion
    /// A type cast: `CAST(expr AS type)`.
    Cast {
        expr: Box<Expr<'a>>,
        as_type: crate::types::DataType,
    },

    // Conditional
    /// A CASE expression: `CASE WHEN ... THEN ... ELSE ... END`.
    Case {
        whens: Vec<(Expr<'a>, Expr<'a>)>,
        else_expr: Option<Box<Expr<'a>>>,
    },

    // Subquery
    /// A subquery: `(SELECT ...)`.
    Subquery(String),

    // Set membership
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

    // Range
    /// `expr [NOT] BETWEEN low AND high`.
    Between {
        expr: Box<Expr<'a>>,
        low: Box<Expr<'a>>,
        high: Box<Expr<'a>>,
        negated: bool,
    },

    // Existence
    /// `[NOT] EXISTS (subquery)`.
    Exists { subquery: String, negated: bool },

    // Null check
    /// `expr IS [NOT] NULL`.
    IsNull { expr: Box<Expr<'a>>, negated: bool },

    // Composites
    /// An object literal: `{ key: value, key2: value2 }`.
    ObjectLiteral(Vec<(String, Expr<'a>)>),
    /// An array literal: `[1, 2, 3]`.
    ArrayLiteral(Vec<Expr<'a>>),

    // Escape hatch
    /// Raw expression string (escape hatch).
    Raw(String),

    // Decoration
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
