//! Arena-native expression node types.
//!
//! [`ExprNode`] is the lifetime-free, flat representation of an expression
//! stored in an [`ExprArena`]. Children are referenced by [`NodeId`] (index
//! into the arena), operators by [`OpId`] (index into the arena's op table),
//! and functions by [`FuncId`] (index into the arena's func table). Identifier
//! strings are represented by [`StrId`] from an [`Interner`].
//!
//! This module is internal — users work with [`Expr<'a>`]; the arena is
//! invisible until [`BuildSession`] wires them together in Phase 5.
//!
//! [`ExprArena`]: super::arena::ExprArena
//! [`Interner`]:  super::interner::Interner
//! [`Expr<'a>`]:  super::Expr
//! [`BuildSession`]: crate::session::BuildSession

use smallvec::SmallVec;

use super::interner::StrId;
use super::literal::Literal;
use super::ops::UnaryOp;
use super::order::{Direction, NullsPosition};
use super::window::WindowFrame;

// ─── Compact indices ─────────────────────────────────────────────────────────

/// Index into an [`ExprArena`]'s node table.
///
/// [`ExprArena`]: super::arena::ExprArena
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub(crate) u32);

/// Index into an [`ExprArena`]'s operator table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpId(pub(crate) u32);

/// Index into an [`ExprArena`]'s function table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncId(pub(crate) u32);

// ─── Path storage ────────────────────────────────────────────────────────────

/// An interned identifier path: 1-3 [`StrId`]s inline, spills to heap beyond.
///
/// Mirrors [`PathExpr`]'s shape but uses interned ids instead of
/// heap-allocated strings.
///
/// [`PathExpr`]: super::path::PathExpr
pub type PathIds = SmallVec<[StrId; 3]>;

// ─── Boxed payloads for large variants ───────────────────────────────────────

/// Payload for [`ExprNode::Func`] — boxed to keep `ExprNode` ≤ 48 bytes.
///
/// `args` uses `SmallVec<[NodeId; 4]>`: functions with ≤ 4 arguments (the
/// overwhelming majority) require no heap allocation; the `SmallVec` itself
/// is the same 24 bytes as a `Vec`, so `FuncNode`'s size is unchanged.
#[derive(Debug, Clone)]
pub struct FuncNode {
    pub id:   FuncId,
    pub args: SmallVec<[NodeId; 4]>,
}

/// Payload for [`ExprNode::Cast`] — boxed to keep `ExprNode` ≤ 48 bytes.
#[derive(Debug, Clone)]
pub struct CastNode {
    pub expr:    NodeId,
    pub as_type: crate::types::DataType,
}

/// Payload for [`ExprNode::Case`] — boxed to keep `ExprNode` ≤ 48 bytes.
///
/// `whens` uses `SmallVec<[(NodeId, NodeId); 2]>`: CASE expressions with ≤ 2
/// WHEN branches (the common case) avoid a heap allocation; size is
/// unchanged at 24 bytes.
#[derive(Debug, Clone)]
pub struct CaseNode {
    pub whens:     SmallVec<[(NodeId, NodeId); 2]>,
    pub else_expr: Option<NodeId>,
}

/// An ORDER BY element that lives inside a [`WindowNode`].
#[derive(Debug, Clone)]
pub struct ArenaOrderBy {
    pub expr:      NodeId,
    pub direction: Direction,
    pub nulls:     Option<NullsPosition>,
}

/// Payload for [`ExprNode::Window`] — boxed to keep `ExprNode` ≤ 48 bytes.
///
/// Both `partition_by` and `order_by` use `SmallVec` with small inline
/// capacities: window functions typically partition on 1–3 keys and sort on
/// 1–2 keys, so the common case is allocation-free.
#[derive(Debug, Clone)]
pub struct WindowNode {
    pub func:         NodeId,
    pub partition_by: SmallVec<[NodeId; 3]>,
    pub order_by:     SmallVec<[ArenaOrderBy; 2]>,
    pub frame:        Option<WindowFrame>,
}

// ─── ExprNode ────────────────────────────────────────────────────────────────

/// A flat, lifetime-free expression node stored in an [`ExprArena`].
///
/// Every sub-expression is referenced by [`NodeId`] rather than by a
/// `Box<Expr>` pointer, enabling contiguous storage and iterative traversal.
///
/// # Size budget
///
/// `size_of::<ExprNode>() <= 48` on 64-bit targets.  The variants that would
/// exceed this budget (Func, Cast, Case, Window) store their payload behind a
/// `Box`.
///
/// [`ExprArena`]: super::arena::ExprArena
#[derive(Debug, Clone)]
pub enum ExprNode {
    // ── References ──────────────────────────────────────────────────────────

    /// A path reference: interned segment ids.
    Ref(PathIds),

    /// Sub-path access on an expression result.
    Access { base: NodeId, path: PathIds },

    // ── Values ───────────────────────────────────────────────────────────────

    /// A positional bind parameter.
    Param,

    /// A literal value (all Cow data owned, no lifetime).
    Value(Literal<'static>),

    /// An array literal.
    ///
    /// Uses `SmallVec<[NodeId; 4]>` (same 24-byte footprint as `Vec`) to
    /// avoid heap allocation for arrays of ≤ 4 elements.
    Array(SmallVec<[NodeId; 4]>),

    /// An object / map literal: interned key ids + child node ids.
    ///
    /// Uses `SmallVec` with 2 inline slots — small literal objects (the common
    /// case) are allocation-free.
    Object(SmallVec<[(StrId, NodeId); 2]>),

    // ── Operations ───────────────────────────────────────────────────────────

    /// A binary operation: child ids + operator index.
    BinaryOp { left: NodeId, op: OpId, right: NodeId },

    /// A unary operation.
    UnaryOp { op: UnaryOp, expr: NodeId },

    // ── Calls ────────────────────────────────────────────────────────────────

    /// A function call — boxed because `FuncDef` (32 B) + `Vec` would exceed
    /// the 48-byte budget.
    Func(Box<FuncNode>),

    // ── Structural ───────────────────────────────────────────────────────────

    /// A type cast — boxed because `DataType` (40 B) + `NodeId` would exceed
    /// the 48-byte budget.
    Cast(Box<CastNode>),

    /// A `CASE WHEN … THEN … ELSE … END` expression.
    Case(Box<CaseNode>),

    /// `expr BETWEEN low AND high`.
    Between { expr: NodeId, low: NodeId, high: NodeId },

    /// `expr NOT BETWEEN low AND high` — canonical arena form.
    ///
    /// Produced by [`lower()`] when it encounters `NOT(Between(…))`, avoiding
    /// the look-ahead required in a post-order scan.
    ///
    /// [`lower()`]: super::arena::ExprArena::lower
    NotBetween { expr: NodeId, low: NodeId, high: NodeId },

    /// `expr IN (list)`.
    InList { expr: NodeId, list: Vec<NodeId> },

    /// `expr NOT IN (list)` — canonical arena form.
    ///
    /// Produced by [`lower()`] when it encounters `NOT(InList(…))`.
    ///
    /// [`lower()`]: super::arena::ExprArena::lower
    NotInList { expr: NodeId, list: Vec<NodeId> },

    // ── Decoration ───────────────────────────────────────────────────────────

    /// `expr AS alias`.
    Alias { expr: NodeId, alias: StrId },

    /// `*` (wildcard projection).
    Star,

    /// `COUNT(*)`.
    CountStar,

    /// A window function.
    Window(Box<WindowNode>),
}
