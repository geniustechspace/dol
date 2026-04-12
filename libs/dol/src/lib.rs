//! # DOL — Data Operating Language
//!
//! A universal, storage-agnostic query and schema language for Rust.
//!
//! ## Overview
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
//! use dol::model::{Model, Field, FieldType};
//! use dol::backend::sql::dialect::Dialect;
//! use dol::builder::ModelBuilderExt;
//! use dol::ToSql;
//!
//! // Define a static model (zero-cost, const-compatible)
//! static USERS: Model = Model::new("users", &[
//!     Field::new("id", FieldType::Uuid).primary_key(),
//!     Field::new("email", FieldType::Text).unique(),
//!     Field::new("status", FieldType::Text).default("'active'"),
//! ]);
//!
//! // Generate SQL for different dialects
//! let pg_sql = USERS.get()
//!     .all_columns()
//!     .where_eq("id")
//!     .to_sql(Some(&Dialect::postgres()));
//! assert!(pg_sql.contains("$1"));
//!
//! let sqlite_sql = USERS.get()
//!     .all_columns()
//!     .where_eq("id")
//!     .to_sql(None);  // Uses default (SQLite)
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

/// Schema language — Model, Field, FieldType, and constraints.
pub use dol_core::model;

/// Intermediate representation — backend-agnostic AST.
pub use dol_core::ir;

/// Builder API — composable method-chain builders that produce IR.
pub use dol_core::builder;

/// Backend implementations.
pub mod backend {
    /// Backend trait and shared output types (from dol-core::ir).
    pub use dol_core::ir::{
        Backend, BackendError, KvOp, KvOutput, RenderedOutput, SqlOutput, StorageOp, StorageOutput,
    };

    /// SQL backend — dialect-aware SQL rendering.
    pub mod sql {
        pub use dol_sql::*;

        // Re-export SqlOutput for backward compat at backend::sql::SqlOutput
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
pub use dol_core::model::constraint::{FkAction, ForeignKeyRef, GeneratedKind, ModelConstraint};
pub use dol_core::model::{Field, FieldType, Model};
pub use dol_sql::dialect::Dialect;

#[cfg(feature = "config")]
pub use dol_config::DolConfig;
#[cfg(feature = "config")]
pub use dol_config::{BackendFilter, LockStrategy, UnifiedMigrationConfig};

// Re-export builder extension traits so users can call model.get(), etc.
pub use dol_core::builder::{ModelBuilderExt, ModelDefineExt};

// Re-export the ToSql extension trait so builders have .to_sql() in scope.
pub use dol_sql::ext::ToSql;

// Re-export TransactionSqlExt so TransactionBuilder::to_sql() works.
pub use dol_sql::ext::TransactionSqlExt;

// Re-export CompoundSelectBuilder (moved from dol-builder to dol-sql).
pub use dol_sql::ext::CompoundSelectBuilder;

// Re-export GetBuilderSqlExt for union/intersect/except/as_scalar on GetBuilder.
pub use dol_sql::ext::GetBuilderSqlExt;

#[cfg(test)]
mod tests;
