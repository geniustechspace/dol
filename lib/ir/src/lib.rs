//! DOL IR — universal `Operation` enum and Backend trait.
//!
//! v2 is a noun/verb hybrid: structural / governance variants are nouns,
//! data / query / authorization variants are verbs. See
//! `docs/rfcs/0001-ir-v2.md` and `docs/IR.md`.
//!
//! v1 callers continue to work via the [`Statement`] surface and the
//! [`compat::statement`] shim until they migrate.
//!
//! Schema constraint types (`RefAction`, `ComputedKind`, `RelationRef`,
//! `EntityConstraint`) live in [`dol_schema`] and are re-exported here for
//! convenience.

#![deny(unsafe_code)]

extern crate alloc;

pub mod backend;
pub mod capabilities;
pub mod compat;
pub mod control;
pub mod definition;
pub mod entity_ref;
pub mod operation;
pub mod prelude;
pub mod program;
pub mod program_ref;
pub mod schema_catalog;
pub mod schema_ref;
pub mod statement;
pub mod storage;
pub mod target;
pub mod transaction;
pub mod version;

/// Schema constraint types — re-exported from `dol-schema`, the canonical home.
pub mod constraint {
    pub use dol_schema::constraint::{ComputedKind, EntityConstraint, RefAction, RelationRef};
}

// ── v2 surface ────────────────────────────────────────────────────────────
pub use backend::{Backend, BackendError};
pub use capabilities::{BackendCapabilities, CapabilityCheck, CapabilitySet, CapabilityTag};
pub use constraint::{ComputedKind, EntityConstraint, RefAction, RelationRef};
pub use operation::{Category, OpKind, Operation};
pub use program::Program;
pub use program_ref::ProgramRef;
pub use schema_catalog::{CatalogEntry, SchemaCatalog, TypeEntry};
pub use schema_ref::{CatalogId, SchemaId, SchemaRef};
pub use target::{Locator, SchemaBinding, Symbol, Target, TargetKind};
pub use version::{IR_SCHEMA_VERSION, VersionedProgram};

// ── v1 surface (preserved during the v1→v2 migration window) ─────────────
//
// These re-exports stay non-deprecated for now: in-tree consumers
// (`dol-query`, `dol-pipeline`, `dol-stream`, etc.) still emit `Statement`
// directly, and the prototype's policy is to migrate them before adding
// `#[deprecated]` markers. The canonical re-export point during the
// migration window is [`compat::statement`].
pub use control::{DefinePolicy, Grant, PolicyAction, Privilege, Revoke};
pub use definition::{
    AlterAction, AlterEntity, DefineEntity, DefineLookup, DefineType, DropEntity, DropLookup,
    DropType, FieldDef, LookupMethod,
};
pub use entity_ref::EntityRef;
pub use storage::{GetObject, ListObjects, MoveFile, ObjectSource, PutObject, ReadFile, WriteFile};
pub use statement::{Statement, StatementExtension};
pub use transaction::Transaction;
