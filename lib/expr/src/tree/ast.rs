//! Expression AST definitions.

use alloc::{boxed::Box, vec::Vec};

use super::compact_name::CompactName;
use super::func::meta::FuncDef;
use super::literal::Literal;
use super::op::UnaryOp;
use super::op::meta::OpDef;
use super::order::OrderByExpr;
use super::path::PathExpr;
use super::window::ContextFrame;

/// A composable expression node, the core AST type for DOL.
///
/// Every expression is backend-agnostic. Backends (SQL, document, KV) interpret
/// and render expressions according to their own semantics.
///
/// The lifetime `'a` allows zero-copy string literals in the AST via
/// [`Literal<'a>`], which uses `Cow<'a, str>` / `Cow<'a, [u8]>` internally.
///
/// # Path expressions
///
/// Container paths and leaf attributes are kept as **distinct** AST nodes:
///
/// - [`Expr::Namespace`] is a *container-only* address (e.g. `users`,
///   `schema.users`, `api/v1/users`, `s3://bucket/prefix`). It never names
///   a leaf attribute.
/// - [`Expr::Field`] is a *leaf-only* reference. It carries a leaf `name`
///   (the attribute), an optional `base` namespace anchoring it, and an
///   optional in-leaf traversal chain (`steps`) for JSON/array access.
///
/// Construction:
/// - [`field("email")`](super::field) → unanchored leaf.
/// - [`namespace("users")`](super::namespace) → container path.
/// - `namespace("users").field("email")` → leaf anchored on namespace.
///
/// # Constructors
///
/// ```rust
/// use dol_expr::tree::{field, string, int};
///
/// let expr = field("age").gt(int(18i32));
/// let expr = field("age").gt(int(18i32)) & field("status").eq(string("active"));
/// let expr = field("profile").get("address").get("city");
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum Expr<'a> {
    // ── References ──────────────────────────────────────────────────────────
    /// A container path (no leaf): `users`, `schema.users`, `api/v1/users`.
    ///
    /// Use [`namespace()`](super::namespace) to construct. To name a leaf
    /// inside a namespace, chain with [`Expr::field()`].
    ///
    /// At the arena level this lowers to `ExprNode::Namespace`.
    Namespace(PathExpr),

    /// A leaf attribute reference, optionally anchored on a [`Namespace`]
    /// (`base`), with an optional in-leaf traversal chain (`steps`) for
    /// JSON / object / array access.
    ///
    /// At the arena level this lowers to `ExprNode::Field`.
    ///
    /// [`Namespace`]: Expr::Namespace
    Field {
        /// Optional anchor — must be a [`Namespace`](Expr::Namespace) when set.
        base: Option<Box<Expr<'a>>>,
        /// Leaf attribute name.
        name: CompactName,
        /// In-leaf traversal chain (each segment is a key access).
        steps: Vec<CompactName>,
    },

    // ── Values ───────────────────────────────────────────────────────────────
    /// A positional bind parameter (`$1`, `?`, `@p1`, `:1`).
    Param,

    /// A literal value from the unified type system.
    Value(Literal<'a>),

    /// An array literal: `[elem1, elem2, ...]`.
    Array(Vec<Expr<'a>>),

    /// An object / map literal: `{ key: expr, ... }`.
    Object(Vec<(CompactName, Expr<'a>)>),

    // ── Operations ───────────────────────────────────────────────────────────
    /// A binary operation: `left op right`.
    ///
    /// Negation (`NOT LIKE`, `NOT IN`, etc.) is expressed by wrapping in
    /// `UnaryOp::Not` via [`Expr::negate()`].
    BinaryOp {
        left: Box<Expr<'a>>,
        op: OpDef,
        right: Box<Expr<'a>>,
    },

    /// A unary operation: `op expr`.
    ///
    /// Includes `NOT`, `-`, `~`, `IS NULL`, `IS NOT NULL`.
    UnaryOp { op: UnaryOp, expr: Box<Expr<'a>> },

    // ── Calls ────────────────────────────────────────────────────────────────
    /// A function call: `name(args…)`.
    Func { name: FuncDef, args: Vec<Expr<'a>> },

    // ── Structural ───────────────────────────────────────────────────────────
    /// A type cast: `CAST(expr AS type)`.
    Cast {
        expr: Box<Expr<'a>>,
        as_type: crate::types::DataType,
    },

    /// A `CASE WHEN … THEN … ELSE … END` expression.
    Case {
        whens: Vec<(Expr<'a>, Expr<'a>)>,
        else_expr: Option<Box<Expr<'a>>>,
    },

    /// `expr [NOT] BETWEEN low AND high`.
    ///
    /// For `NOT BETWEEN`, wrap with [`Expr::negate()`].
    Between {
        expr: Box<Expr<'a>>,
        low: Box<Expr<'a>>,
        high: Box<Expr<'a>>,
    },

    /// `expr [NOT] IN (list)`.
    ///
    /// For `NOT IN`, wrap with [`Expr::negate()`].
    InList {
        expr: Box<Expr<'a>>,
        list: Vec<Expr<'a>>,
    },

    // ── Decoration ───────────────────────────────────────────────────────────
    /// `expr AS alias`.
    Alias {
        expr: Box<Expr<'a>>,
        alias: CompactName,
    },

    /// `*` (all fields in a projection).
    Star,

    /// `COUNT(*)`.
    CountStar,

    /// A window function: `func OVER (PARTITION BY … ORDER BY … frame)`.
    Window {
        func: Box<Expr<'a>>,
        partition_by: Vec<Expr<'a>>,
        order_by: Vec<OrderByExpr<'a>>,
        frame: Option<ContextFrame>,
    },
}
