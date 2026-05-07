//! Schema catalog handles — re-exports of the canonical homes in
//! [`dol_core::schema`].
//!
//! The handle types ([`CatalogId`], [`SchemaId`], [`SchemaRef`]) live in
//! `dol-core` so downstream IR crates can reference them without
//! depending on `dol-schema`. They are re-exported here at their
//! historical paths for backwards compatibility.

pub use dol_core::schema::{CatalogId, SchemaId, SchemaRef};
