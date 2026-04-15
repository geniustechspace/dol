//! # DOL — Data Operating Language
//!
//! A universal, storage-agnostic query and schema language for Rust.
//!
//! ## Overview

#![deny(unsafe_code)]
//!
//! DOL provides a type-safe, composable way to build queries and schema definitions
//! that can target multiple storage backends: SQL databases, key-value stores,
//! document databases, and object storage.
//!
//! ## Architecture
//!
//! DOL uses a three-layer pipeline:
//!
//! ```text
//! Layer 1: Builders (Human-friendly API)
//!       ↓
//! Layer 2: IR (Intermediate Representation — backend-agnostic AST)
//!       ↓
//! Layer 3: Backends (SQL, KV, Document, Object Storage)
//! ```
//!
//! ## Crate Structure
//!
//! This is the umbrella crate that re-exports from focused sub-crates:
//!
//! - **`dol-core`** — The complete language layer (expressions, models, IR, builders)
//! - **`dol-sql`** — SQL renderer + dialect system
//! - **`dol-kv`** — Key-value backend
//! - **`dol-objects`** — Object storage backend
//! - **`dol-migration`** — Migration system (feature-gated)
//! - **`dol-config`** — Unified configuration (feature-gated)
//!
//! ## Quick Start
//!
//! ```rust
//! use dol::model::{Entity, Field, FieldType};
//! use dol::backend::sql::dialect::Dialect;
//! use dol::builder::EntityBuilderExt;
//! use dol::Render;
//!
//! // Define a static model (zero-cost, const-compatible)
//! static USERS: Entity = Entity::new("users", &[
//!     Field::new("id", FieldType::Uuid).primary_key(),
//!     Field::new("email", FieldType::Text).unique(),
//!     Field::new("status", FieldType::Text).default("'active'"),
//! ]);
//!
//! // Generate SQL for different dialects
//! let pg_sql = USERS.get()
//!     .all_fields()
//!     .where_eq("id")
//!     .render(Some(&Dialect::postgres())).unwrap();
//! assert!(pg_sql.contains("$1"));
//!
//! let sqlite_sql = USERS.get()
//!     .all_fields()
//!     .where_eq("id")
//!     .render(None).unwrap();  // Uses default (SQLite)
//! assert!(sqlite_sql.contains("?"));
//! ```
//!
//! ## Supported Backends
//!
//! - **SQL**: PostgreSQL, MySQL, MariaDB, SQLite, MSSQL, Oracle, CockroachDB
//! - **Key-Value**: Abstract KV operations (get, put, delete, list)
//! - **Object Storage**: S3-compatible operations (put, get, list objects)

// ── Sub-crate re-exports (preserving the original module paths) ──

/// The complete language layer — expressions, models, IR, builders, Backend trait.
/// Users can also depend on `dol-core` directly for a standalone, engine-free experience.
pub use dol_core as language;

/// Expression engine — composable, backend-agnostic expression AST.
pub use dol_core::expr;

/// Schema language — Entity, Field, FieldType, and constraints.
pub use dol_core::model;

/// Intermediate representation — backend-agnostic AST.
pub use dol_core::ir;

/// Builder API — composable method-chain builders that produce IR.
pub use dol_core::builder;

/// Query entry point — backend-neutral query construction from entities or strings.
pub use dol_core::query;

/// Backend implementations.
pub mod backend {
    /// Backend trait and shared output types (from dol-core::ir).
    pub use dol_core::ir::{
        Backend, BackendError, KvOp, KvOutput, RenderedOutput, SqlOutput, StorageOp, StorageOutput,
    };

    /// SQL backend — dialect-aware SQL rendering.
    pub mod sql {
        pub use dol_sql::*;

        pub use dol_core::ir::SqlOutput;
    }

    /// Key-value backend.
    pub mod kv {
        pub use dol_kv::*;
    }

    /// Object storage backend.
    pub mod storage {
        pub use dol_objects::*;
    }
}

/// Migration system (feature-gated).
#[cfg(feature = "migration")]
pub use dol_migration as migration;

/// Unified configuration (feature-gated).
#[cfg(feature = "config")]
pub use dol_config as config;

// ── Top-level re-exports for ergonomic use ──

pub use dol_core::ir::definition::FieldDef;
pub use dol_core::model::constraint::{EntityConstraint, FkAction, ForeignKeyRef, GeneratedKind};
pub use dol_core::model::{Entity, Field, FieldType};
pub use dol_sql::dialect::Dialect;

#[cfg(feature = "config")]
pub use dol_config::DolConfig;
#[cfg(feature = "config")]
pub use dol_config::{BackendFilter, LockStrategy, UnifiedMigrationConfig};

// Re-export builder extension traits so users can call model.get(), etc.
pub use dol_core::builder::{EntityBuilderExt, EntityDefineExt};

// Re-export commonly used builders at the top level for ergonomic access.
pub use dol_core::builder::{DefinePolicyBuilder, DefineTypeBuilder, DropTypeBuilder};

// Re-export Query for backend-neutral entry point.
pub use dol_core::query::Query;

// Re-export the Render extension trait so builders have .render() in scope.
pub use dol_sql::ext::Render;

// Re-export TransactionRender so TransactionBuilder::render() works.
pub use dol_sql::ext::TransactionRender;

// Re-export CompoundSelectBuilder (moved from dol-builder to dol-sql).
pub use dol_sql::ext::CompoundSelectBuilder;

// Re-export GetBuilderSqlExt for union/intersect/except/as_scalar on GetBuilder.
pub use dol_sql::ext::GetBuilderSqlExt;

#[cfg(test)]
mod tests;
