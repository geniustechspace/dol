//! Backward compatibility — re-exports from [`crate::op::query`].

pub use crate::op::query::*;

pub type QueryIR<'a> = crate::op::Query<'a>;
pub type CompoundQueryIR<'a> = crate::op::CompoundQuery<'a>;
pub type JoinIR = crate::op::Join;
pub type JoinType = crate::op::JoinKind;
pub type SetOpKind = crate::op::SetOp;
