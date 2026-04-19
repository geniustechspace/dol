//! Backward compatibility — re-exports from [`crate::op::control`].

pub use crate::op::control::*;

pub type GrantIR = crate::op::Grant;
pub type RevokeIR = crate::op::Revoke;
pub type DefinePolicyIR<'a> = crate::op::DefinePolicy<'a>;
