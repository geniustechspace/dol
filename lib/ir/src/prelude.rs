//! Prelude — the curated public surface of `dol-ir`.

pub use crate::backend::{Backend, BackendError};
pub use crate::capabilities::{
    BackendCapabilities, CapabilityCheck, CapabilitySet, CapabilityTag,
};
pub use crate::operation::{Category, OpKind, Operation};
pub use crate::privilege::Privilege;
pub use crate::program::Program;
pub use crate::program_ref::ProgramRef;
pub use crate::schema_catalog::{CatalogEntry, SchemaCatalog, TypeEntry};
pub use crate::schema_ref::{CatalogId, SchemaId, SchemaRef};
pub use crate::target::{Locator, SchemaBinding, Symbol, Target, TargetKind};
pub use crate::version::{IR_SCHEMA_VERSION, VersionedProgram};
