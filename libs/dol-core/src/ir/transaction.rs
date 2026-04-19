//! Backward compatibility — re-exports from [`crate::op::transaction`].

pub use crate::op::transaction::*;

pub type TransactionIR<'a> = crate::op::Transaction<'a>;
