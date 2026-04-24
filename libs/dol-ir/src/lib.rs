//! DOL IR — complete Statement enum covering all DOL operation types.
//!
//! This crate is the single IR layer for all DOL backends.  Contains
//! the canonical Statement enum, Backend trait, and DDL/DML node types.
//!
//! Schema constraint types (`FkAction`, `GeneratedKind`, `ForeignKeyRef`,
//! `EntityConstraint`) live in [`dol_schema`] and are re-exported here for
//! convenience.

#![deny(unsafe_code)]

pub mod backend;
pub mod control;
pub mod definition;
pub mod entity_ref;
pub mod prelude;
pub mod statement;
pub mod storage;
pub mod transaction;

/// Schema constraint types — re-exported from `dol-schema`, the canonical home.
pub mod constraint {
    pub use dol_schema::constraint::{EntityConstraint, FkAction, ForeignKeyRef, GeneratedKind};
}

pub use backend::{Backend, BackendError};
pub use constraint::{EntityConstraint, FkAction, ForeignKeyRef, GeneratedKind};
pub use control::{DefinePolicy, Grant, PolicyAction, Privilege, Revoke};
pub use definition::{
    AlterAction, AlterEntity, DefineEntity, DefineIndex, DefineType, DropEntity, DropIndex,
    DropType, FieldDef, IndexMethod,
};
pub use entity_ref::EntityRef;
pub use statement::Statement;
pub use storage::{GetObject, ListObjects, MoveFile, ObjectSource, PutObject, ReadFile, WriteFile};
pub use transaction::Transaction;
