//! DOL expression engine — arena-based IR and user-facing tree DSL.
//!
//! This crate provides two expression representations:
//!
//! - **`tree::Expr<'a>`** — A recursive tree AST with a fluent builder API.
//!   This is what users compose expressions with (`field("x").eq(param())`).
//!
//! - **`ExprNode`** — A flat, arena-based node (≤ 32 bytes) for efficient
//!   storage and backend processing. Tree expressions are lowered into this
//!   form before rendering.

#![deny(unsafe_code)]

pub mod arena;
pub mod expr;
pub mod ids;
pub mod interner;
pub mod session;
pub mod tree;
pub mod types;

pub use arena::{
    CaseNode, ExprArena, FieldNode, FieldStep, FuncNode, InListNode, ObjLitNode, Span, SpanTable,
    WindowNode,
};
pub use expr::{
    BinOp, ConflictClause, DeleteNode, ExprNode, InsertNode, JoinNode, JoinType, LockHint,
    Order, QueryNode, UnaryOp, UpdateNode, UpsertNode,
};
pub use ids::{
    CaseId, DeleteId, FieldId, FuncId, InListId, InsertId, LiteralId, NodeId, ObjLitId, QueryId,
    SpanId, StrId, TypeId, UpdateId, UpsertId, WindowId, NULL_NODE,
};
pub use interner::Interner;
pub use session::BuildSession;
pub use types::{DataType, Literal, Value};
