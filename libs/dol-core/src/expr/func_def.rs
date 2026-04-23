//! Compatibility barrel for function and operator metadata.
//!
//! The focused implementations now live in smaller sibling modules, but this
//! module keeps the existing `expr::func_def::*` import path stable.

pub use super::compact_name::CompactName;
pub use super::func_meta::{Arity, ArityError, DolFunc, FuncDef, FuncKind};
pub use super::op_meta::{DolOp, OpDef, OpKind};
