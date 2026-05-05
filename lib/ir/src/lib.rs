//! DOL IR — universal `Operation` enum and Backend trait.
//!
//! The IR is a noun/verb hybrid: structural / governance variants are nouns
//! (`Schema`, `Field`, `Index`, `Lookup`, `Policy`, `Mask`, `Quota`,
//! `Audit`) and carry a [`StructuralVerb`](operation::StructuralVerb).
//! Data / query / authorization variants are verbs (`Insert`, `Update`,
//! `Replace`, `Delete`, `Upsert`, `Append`, `Query`, `Probe`, `Describe`,
//! `Grant`, `Revoke`). Meta variants (`Tx`, `Extension`, feature-gated
//! `Raw`) round it out.
//!
//! See `docs/IR.md` for the design overview and `docs/rfcs/0001-ir.md`
//! for the rationale.
//!
//! Schema constraint types (`RefAction`, `ComputedKind`, `RelationRef`,
//! `EntityConstraint`) live in [`dol_schema`] and are re-exported here for
//! convenience.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

pub mod backend;
pub mod capabilities;
pub mod operation;
pub mod prelude;
pub mod privilege;
pub mod program;
pub mod program_ref;
pub mod schema_catalog;
pub mod schema_ref;
pub mod target;

/// Schema constraint types — re-exported from `dol-schema`, the canonical home.
pub mod constraint {
    pub use dol_schema::constraint::{ComputedKind, EntityConstraint, RefAction, RelationRef};
}

pub use backend::{Backend, BackendError};
pub use capabilities::{BackendCapabilities, CapabilityCheck, CapabilitySet, CapabilityTag};
pub use constraint::{ComputedKind, EntityConstraint, RefAction, RelationRef};
pub use operation::{Category, OpKind, Operation};
pub use privilege::Privilege;
pub use program::Program;
pub use program_ref::ProgramRef;
pub use schema_catalog::{CatalogEntry, SchemaCatalog, TypeEntry};
pub use schema_ref::{CatalogId, SchemaId, SchemaRef};
pub use target::{Locator, SchemaBinding, Symbol, Target, TargetKind};
