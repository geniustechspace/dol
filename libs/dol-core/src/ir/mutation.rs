//! Backward compatibility — re-exports from [`crate::op::mutation`].

pub use crate::op::mutation::*;

pub type InsertIR = crate::op::Insert;
pub type InsertSelectIR = crate::op::InsertSelect;
pub type UpdateIR<'a> = crate::op::Update<'a>;
pub type RemoveIR<'a> = crate::op::Remove<'a>;
pub type UpsertIR<'a> = crate::op::Upsert<'a>;
