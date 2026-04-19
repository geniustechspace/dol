//! Backward compatibility — re-exports from [`crate::op::definition`].

pub use crate::op::definition::*;

pub type DefineEntityIR = crate::op::DefineEntity;
pub type AlterEntityIR = crate::op::AlterEntity;
pub type DropEntityIR = crate::op::DropEntity;
pub type DefineIndexIR = crate::op::DefineIndex;
pub type DropIndexIR = crate::op::DropIndex;
pub type DefineTypeIR = crate::op::DefineType;
pub type DropTypeIR = crate::op::DropType;
