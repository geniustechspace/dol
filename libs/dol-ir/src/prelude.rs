//! Curated re-exports — the stable, recommended surface for downstream users.
//!
//! ```ignore
//! use dol_ir::prelude::*;
//! ```

pub use crate::backend::{Backend, BackendError};
pub use crate::constraint::{ComputedKind, EntityConstraint, RefAction, RelationRef};
pub use crate::control::{DefinePolicy, Grant, PolicyAction, Privilege, Revoke};
pub use crate::definition::{
    AlterAction, AlterEntity, DefineEntity, DefineLookup, DefineType, DropEntity, DropLookup,
    DropType, FieldDef, LookupMethod,
};
pub use crate::entity_ref::EntityRef;
pub use crate::statement::Statement;
pub use crate::storage::{
    GetObject, ListObjects, MoveFile, ObjectSource, PutObject, ReadFile, WriteFile,
};
pub use crate::transaction::Transaction;
