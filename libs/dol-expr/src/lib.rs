//! DOL expression AST, type system, arena, and interner (internal).

#![deny(unsafe_code)]

pub mod arena;
pub mod expr;
pub mod ids;
pub mod interner;
pub mod session;
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
