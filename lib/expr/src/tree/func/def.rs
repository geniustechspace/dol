//! Re-export barrel for function and operator definition shapes.
//!
//! The focused implementations live in:
//! - `tree::compact_name`            — `CompactName`
//! - `tree::func::meta`               — `Arity`, `ArityError`, `DolFunc`, `FuncDef`, `FuncKind`
//! - `tree::op::meta`                 — `DolOp`, `OpDef`, `OpCategory`
//!
//! This module keeps the historical `tree::func::def::*` import path stable
//! for downstream macros and consumers.

pub use super::super::compact_name::CompactName;
pub use super::super::op::meta::{DolOp, OpCategory, OpDef};
pub use super::meta::{Arity, ArityError, DolFunc, FuncDef, FuncKind};
