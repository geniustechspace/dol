//! DOL IR — complete Statement enum covering all DOL operation types.
//!
//! This crate is the single IR layer for all DOL backends.  Contains
//! the canonical Statement enum, Backend trait, and DDL/DML node types.
//!
//! Schema constraint types (`RefAction`, `ComputedKind`, `RelationRef`,
//! `EntityConstraint`) live in [`dol_schema`] and are re-exported here for
//! convenience.

#![deny(unsafe_code)]

pub mod backend;
pub mod capabilities;
pub mod control;
pub mod definition;
pub mod entity_ref;
pub mod prelude;
pub mod program;
pub mod schema_ref;
pub mod statement;
pub mod storage;
pub mod target;
pub mod transaction;

/// Schema constraint types — re-exported from `dol-schema`, the canonical home.
pub mod constraint {
    pub use dol_schema::constraint::{ComputedKind, EntityConstraint, RefAction, RelationRef};
}

pub use backend::{Backend, BackendError};
pub use capabilities::BackendCapabilities;
pub use constraint::{ComputedKind, EntityConstraint, RefAction, RelationRef};
pub use control::{DefinePolicy, Grant, PolicyAction, Privilege, Revoke};
pub use definition::{
    AlterAction, AlterEntity, DefineEntity, DefineLookup, DefineType, DropEntity, DropLookup,
    DropType, FieldDef, LookupMethod,
};
pub use entity_ref::EntityRef;
pub use program::Program;
pub use schema_ref::{CatalogId, SchemaId, SchemaRef};
pub use statement::{Statement, StatementExtension};
pub use storage::{GetObject, ListObjects, MoveFile, ObjectSource, PutObject, ReadFile, WriteFile};
pub use target::{Locator, SchemaBinding, Symbol, Target, TargetKind};
pub use transaction::Transaction;
