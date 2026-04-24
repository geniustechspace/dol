//! Curated re-exports — the stable, recommended surface for downstream users.
//!
//! ```ignore
//! use dol_expr::prelude::*;
//! ```

pub use crate::arena::{
    CaseNode, ExprArena, FieldNode, FieldStep, FuncNode, InListNode, ObjLitNode, Span, SpanTable,
    WindowNode,
};
pub use crate::expr::{
    BinOp, ConflictClause, DeleteNode, ExprNode, InsertNode, JoinNode, JoinType, LockHint, Order,
    QueryNode, UnaryOp, UpdateNode, UpsertNode,
};
pub use crate::ids::{
    CaseId, DeleteId, FieldId, FuncId, InListId, InsertId, LiteralId, NULL_NODE, NodeId, ObjLitId,
    QueryId, SpanId, StrId, TypeId, UpdateId, UpsertId, WindowId,
};
pub use crate::interner::Interner;
pub use crate::session::BuildSession;
pub use crate::types::{DataType, Literal, Value};

pub use crate::tree::{
    Expr, FuncDef, OpDef, OrderByExpr, PathExpr, UnaryOp as TreeUnaryOp, arr, bool_expr, case,
    field, field_dyn, float, int, null, obj, param, qualified, string,
};
