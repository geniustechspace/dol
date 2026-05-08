//! Prelude — the curated public surface of `dol-command`.
//!
//! The flat `dol_command::prelude::*` import keeps the existing one-line
//! "give me everything" ergonomic. The submodules
//! ([`dml`], [`ddl`], [`dql`], [`acl`], [`tx`]) mirror the
//! `dol_command::operation` namespace split, so consumers can opt into a scoped
//! import — `use dol_command::prelude::dml::*;` — when they only need one
//! verb family. The shared address space ([`Operation`], [`Program`],
//! [`Target`], …) is always re-exported at the top level.

pub use crate::backend::{Backend, BackendError};
pub use crate::capabilities::{BackendCapabilities, CapabilityCheck, CapabilitySet, CapabilityTag};
pub use crate::operation::{Category, OpKind, Operation};
pub use crate::privilege::Privilege;
pub use crate::program::Program;
pub use crate::program_ref::ProgramRef;
pub use crate::target::{Locator, SchemaBinding, Symbol, Target, TargetKind};

pub use dol_schema::{
    CatalogEntry, CatalogId, SchemaCatalog, SchemaId, SchemaRef, TypeBody, TypeEntry,
};

/// Data-manipulation verbs (`Insert`, `Update`, `Replace`, `Delete`,
/// `Upsert`, `Append`).
pub mod dml {
    pub use crate::operation::{
        Append, Delete, Insert, InsertSource, Replace, ReplaceBody, Update, Upsert,
    };
}

/// Data-definition verbs (`Schema`, `Field`, `Index`, `Lookup`).
pub mod ddl {
    pub use crate::operation::{
        FieldDef, FieldOp, IndexDirection, IndexKey, IndexMethod, IndexOp, LookupMethod, LookupOp,
        SchemaBody, SchemaOp,
    };
    // Re-export TypeBody from dol_schema (canonical home).
    pub use dol_schema::TypeBody;
}

/// Data-query verbs (`Query`, `Probe`, `Describe`).
pub mod dql {
    pub use crate::operation::{Describe, DescribeFacet, Probe, Query};
}

/// Access-control verbs (`Grant`, `Revoke`, `Policy`, `Mask`, `Quota`,
/// `Audit`).
pub mod acl {
    pub use crate::operation::{
        AuditEvent, AuditOp, AuditSink, Grant, MaskOp, PolicyOp, PolicyScope, QuotaKind, QuotaOp,
        Revoke,
    };
    pub use crate::privilege::Privilege;
}

/// Transaction control (`Tx`).
pub mod tx {
    pub use crate::operation::{IsolationLevel, TxBegin, TxOp, TxOptions};
}

/// Lower the fluent [`dol_query`] DSL builders to a [`Program`]. Only
/// available when `dol-command` is built with `feature = "query"`
/// (default-on).
#[cfg(feature = "query")]
pub mod query {
    pub use crate::lower_query::{
        BuildError, BuildProgram, lower_delete, lower_get, lower_insert, lower_update, lower_upsert,
    };
}
