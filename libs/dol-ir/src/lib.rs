//! DOL IR — complete Statement enum covering all DOL operation types.
//!
//! This crate is the single IR layer for all DOL backends.  Replaces the
//! old `dol-core::op` module which carried `Expr<'a>` lifetimes.

#![deny(unsafe_code)]

pub mod backend;
pub mod control;
pub mod definition;
pub mod entity_ref;
pub mod statement;
pub mod storage;
pub mod transaction;

pub use backend::{Backend, BackendError};
pub use control::{DefinePolicy, Grant, PolicyAction, Privilege, Revoke};
pub use definition::{
    AlterAction, AlterEntity, Constraint, DefineEntity, DefineIndex, DefineType, DropEntity,
    DropIndex, DropType, FieldDef, ForeignKeyDef, IndexMethod, OwnedEntityConstraint,
    OwnedForeignKeyRef,
};
pub use entity_ref::EntityRef;
pub use statement::Statement;
pub use storage::{GetObject, ListObjects, MoveFile, ObjectSource, PutObject, ReadFile, WriteFile};
pub use transaction::Transaction;
