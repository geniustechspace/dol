//! Tree-based expression AST — the user-facing DSL for composing expressions.
//!
//! This module provides `Expr<'a>`, a recursive tree AST with a fluent builder
//! API. Expressions built here are lowered into the arena-based `ExprNode`
//! representation via `lower.rs` before being passed to backends.
//!
//! # Constructors
//!
//! ```rust
//! use dol_expr::tree::{field, string, int, param};
//!
//! let expr = field("email");
//! let expr = string("active");
//! let expr = int(42i32);
//! let expr = field("age").gt(int(18i32)) & field("status").eq(string("active"));
//! let _ = param();
//! ```

mod ast;
pub mod compact_name;
mod constructors;
mod dsl;
pub mod func;
pub mod func_def;
mod func_meta;
pub mod func_registry;
mod literal;
mod op_meta;
pub mod op_registry;
mod ops;
pub mod order;
mod path;
pub mod window;

pub use ast::Expr;
pub use constructors::{
    IntoFloatLiteral, IntoIntLiteral, arr, bool_expr, case, field, field_dyn, float, int, null,
    obj, param, qualified, string,
};
pub use func_def::{
    Arity, ArityError, CompactName, DolFunc, DolOp, FuncDef, FuncKind, OpDef, OpKind,
};
pub use literal::{Literal, TypeError, Value};
pub use ops::UnaryOp;
pub use order::{Direction, NullsPosition, OrderByExpr};
pub use path::PathExpr;
pub use window::{CaseBuilder, FrameBound, FrameKind, WindowBuilder, WindowFrame};
