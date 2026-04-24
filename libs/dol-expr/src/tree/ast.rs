//! Expression AST definitions.

use super::compact_name::CompactName;
use super::func::meta::FuncDef;
use super::literal::Literal;
use super::op::meta::OpDef;
use super::op::UnaryOp;
use super::order::OrderByExpr;
use super::path::PathExpr;
use super::window::WindowFrame;

/// A composable expression node, the core AST type for DOL.
///
/// Every expression is backend-agnostic. Backends (SQL, document, KV) interpret
/// and render expressions according to their own semantics.
///
/// The lifetime `'a` allows zero-copy string literals in the AST via
/// [`Literal<'a>`], which uses `Cow<'a, str>` / `Cow<'a, [u8]>` internally.
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Expr<'a> {
    // ── References ──────────────────────────────────────────────────────────
    /// A namespace path reference: `field`, `table.field`, `schema.table.field`.
    ///
    /// Use [`field()`] for single-segment, [`qualified()`] for multi-segment.
    /// Rendered by backends as a dotted identifier path.
    ///
    /// At the arena level this is lowered to either `ExprNode::Namespace` or
    /// `ExprNode::Field` depending on segment count and context.
    Namespace(PathExpr),

    /// Sub-path / field traversal on an expression result: `expr.key`,
    /// `expr.a.b`.
    ///
    /// Built by chaining [`Expr::get()`]. For SQL this typically renders as
    /// JSON access (`->>`, `json_extract`, etc.) depending on the dialect.
    Field { base: Box<Expr<'a>>, path: PathExpr },

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
        frame: Option<WindowFrame>,
    },
}
