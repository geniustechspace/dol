//! Composable, backend-agnostic expression AST for DOL.
//!
//! Expressions are the universal building blocks used across all DOL domains
//! (queries, mutations, definitions, etc.). They are backend-agnostic and can
//! be rendered to SQL, document queries, or any other target.
//!
//! # Constructors
//!
//! ```rust
//! use dol_core::expr::{field, string, int, param};
//!
//! let expr = field("email");
//! let expr = string("active");
//! let expr = int(42i32);
//! let expr = field("age").gt(int(18i32)) & field("status").eq(string("active"));
//! let _ = param();
//! ```

pub mod arena;
mod ast;
mod compact_name;
pub mod compiler;
mod constructors;
mod dsl;
pub mod func;
pub mod func_def;
mod func_meta;
pub mod func_registry;
pub mod interner;
mod literal;
mod node;
mod op_meta;
pub mod op_registry;
mod ops;
pub mod order;
mod path;
pub mod pass;
pub mod window;

pub use ast::Expr;
pub use compiler::{ExprRenderer, MAX_EXPR_DEPTH, compile_expr};
pub use constructors::{
    IntoFloatLiteral, IntoIntLiteral, arr, bool_expr, case, field, field_dyn, float, int, null,
    obj, param, qualified, string,
};
pub use path::PathExpr;
pub use func_def::{
    Arity, ArityError, CompactName, DolFunc, DolOp, FuncDef, FuncKind, OpDef, OpKind,
};
pub use literal::{Literal, TypeError, Value};
pub use ops::UnaryOp;
pub use order::{Direction, NullsPosition, OrderByExpr};
pub use window::{CaseBuilder, FrameBound, FrameKind, WindowBuilder, WindowFrame};

#[cfg(test)]
mod tests;
