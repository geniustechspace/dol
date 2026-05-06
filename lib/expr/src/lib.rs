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
//!
//! # Cargo features
//!
//! | feature | default | effect                                                                          |
//! | ------- | :-----: | ------------------------------------------------------------------------------- |
//! | `std`   |         | Forwards `std` to `dol-core`. Disable for `no_std + alloc` targets (default).  |
//! | `serde` |         | `Serialize` for every AST/arena type and `Interner`. v2 wire-in goes through `dol-wire::Decode`. |
//!
//! Building with the default feature set already produces a `no_std + alloc`
//! library suitable for embedded targets.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )
)]
#![warn(missing_docs)]

extern crate alloc;

#[allow(missing_docs)] // tracking: docs follow-up
pub mod arena;
#[allow(missing_docs)] // tracking: docs follow-up
pub mod expr;
pub mod ids;
#[allow(missing_docs)] // tracking: docs follow-up
pub mod interner;
#[allow(missing_docs)] // tracking: docs follow-up
pub mod lower;
pub mod prelude;
#[allow(missing_docs)] // tracking: docs follow-up
pub mod session;
pub mod stats;
#[allow(missing_docs)] // tracking: docs follow-up
pub mod tree;
pub mod types;

pub use arena::{
    Capacity, CaseNode, ExprArena, FieldNode, FieldStep, FuncNode, InListNode, ObjLitNode, Span,
    SpanTable, WindowNode,
};
pub use expr::{
    BinOp, ConflictClause, DeleteNode, ExprNode, InsertNode, JoinNode, JoinType, LockHint, Order,
    QueryNode, UnaryOp, UpdateNode, UpsertNode,
};
pub use ids::{
    CaseId, DeleteId, FieldId, FuncId, InListId, InsertId, LiteralId, NodeId, ObjLitId, QueryId,
    SpanId, StrId, TypeId, UpdateId, UpsertId, WindowId,
};
pub use interner::{InternError, Interner};
pub use lower::{LowerError, lower_expr, lower_expr_with_budget};
pub use session::BuildSession;
pub use types::{DataType, Literal, Value};

// Re-export the core foundation primitives `dol-expr` consumers need so
// they don't have to reach across crates for the most common imports.
pub use dol_core::policy::{Budget, Limits};
