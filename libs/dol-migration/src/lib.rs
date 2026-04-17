//! Enterprise migration system for all DOL storage kinds.
//!
//! This module provides a complete, type-safe migration framework that works
//! across all DOL backends: SQL databases, key-value stores, and object storage.
//!
//! # Design Philosophy
//!
//! Migrations are written in **Rust** using DOL IR — not raw SQL files. This gives:
//! - **Type safety**: The compiler catches schema errors before migration runs.
//! - **Backend agnosticism**: The same migration renders to any SQL dialect.
//! - **Full coverage**: SQL DDL, KV namespaces, and object storage buckets in
//!   one unified system.
//! - **Bidirectional**: Every migration defines both `up()` (forward) and
//!   `down()` (rollback) steps.
//!

#![deny(unsafe_code)]
//! # Architecture
//!
//! ```text
//! ┌──────────────┐     ┌──────────────┐     ┌────────────────┐
//! │  Migration    │────▷│  Runner      │────▷│  Rendered      │
//! │  (Rust code)  │     │  (plan +     │     │  Steps         │
//! │  up() / down()│     │   render)    │     │  (SQL/KV/Stor) │
//! └──────────────┘     └──────┬───────┘     └────────────────┘
//!                             │
//!                      ┌──────▼───────┐
//!                      │  Registry    │
//!                      │  (tracking)  │
//!                      └──────────────┘
//! ```
//!
//! # Quick Start
//!
//! ```rust
//! use dol_migration::{Migration, MigrationStep, MigrationRunner, InMemoryRegistry};
//! use dol_core::ir::definition::FieldDef;
//! use dol_entity::DataType;
//! use dol_core::ir::Statement;
//!
//! // Define a migration in pure Rust
//! struct CreateUsersTable;
//!
//! impl Migration for CreateUsersTable {
//!     fn version(&self) -> &str { "20240101_000001" }
//!     fn description(&self) -> &str { "Create users table" }
//!
//!     fn up(&self) -> Vec<MigrationStep> {
//!         vec![MigrationStep::define_entity(
//!             dol_entity::DefineEntityBuilder::new("users")
//!                 .field(FieldDef::new("id", DataType::Uuid).primary_key())
//!                 .field(FieldDef::new("email", DataType::Text).unique())
//!                 .if_not_exists()
//!                 .build()
//!         )]
//!     }
//!
//!     fn down(&self) -> Vec<MigrationStep> {
//!         vec![MigrationStep::drop_entity("users")]
//!     }
//! }
//!
//! // Set up runner and registry
//! let registry = InMemoryRegistry::new();
//! let runner = MigrationRunner::new()
//!     .register(CreateUsersTable);
//!
//! // Plan forward migration
//! let plan = runner.plan_forward(&registry).unwrap();
//! assert_eq!(plan.steps().len(), 1);
//!
//! // Render to SQL
//! let rendered = runner.render_plan(&plan, None);
//! assert!(rendered[0].steps[0].sql().unwrap().contains("CREATE TABLE"));
//! ```

pub mod async_registry;
mod async_runner;
pub mod migrations;
pub mod plan;
pub mod registry;
pub mod runner;
pub mod schema_diff;

pub use async_registry::{AsyncMigrationRegistry, InMemoryAsyncRegistry};
pub use plan::{MigrationDirection, MigrationPlan, MigrationTarget, PlannedStep};
pub use registry::{AppliedMigration, InMemoryRegistry, migration_history, MigrationRegistry};
pub use runner::{
    MigrationRunner, MigrationState, MigrationStatus, RenderedMigration, RenderedStep,
};
pub use schema_diff::{
    EntitySnapshot, FieldSnapshot, create_entity_step, diff_entities, diff_to_steps,
    drop_entity_step, field_to_field_def,
};

use dol_core::ir;

use std::fmt;

// ===========================================================================
// Migration trait — the user-facing interface
// ===========================================================================

/// A single migration with forward (up) and backward (down) operations.
///
/// Implement this trait to define a migration. Each migration has a unique
/// version string, a human-readable description, and a set of steps for
/// both forward and backward directions.
///
/// # Version Naming
///
/// Versions are sorted lexicographically. Recommended formats:
/// - Timestamp-based: `"20240101_000001"`, `"20240315_120000"`
/// - Sequential: `"V001"`, `"V002"`, `"V003"`
///
/// # Example
///
/// ```rust
/// use dol_migration::{Migration, MigrationStep};
/// use dol_core::ir::definition::FieldDef;
/// use dol_entity::DataType;
///
/// struct AddProfileColumn;
///
/// impl Migration for AddProfileColumn {
///     fn version(&self) -> &str { "20240201_000001" }
///     fn description(&self) -> &str { "Add profile JSON column to users" }
///
///     fn up(&self) -> Vec<MigrationStep> {
///         vec![MigrationStep::alter_entity(
///             "users",
///             vec![dol_core::ir::AlterAction::AddField(
///                 FieldDef::new("profile", DataType::Json).nullable()
///             )]
///         )]
///     }
///
///     fn down(&self) -> Vec<MigrationStep> {
///         vec![MigrationStep::alter_entity(
///             "users",
///             vec![dol_core::ir::AlterAction::DropField("profile".into())]
///         )]
///     }
/// }
/// ```
pub trait Migration: Send + Sync {
    /// Unique version identifier for ordering and tracking.
    fn version(&self) -> &str;

    /// Human-readable description of what this migration does.
    fn description(&self) -> &str;

    /// Forward migration steps (apply schema changes).
    fn up(&self) -> Vec<MigrationStep>;

    /// Backward migration steps (revert schema changes).
    fn down(&self) -> Vec<MigrationStep>;
}

// ===========================================================================
// MigrationStep — wraps IR operations for all storage kinds
// ===========================================================================

/// A single step within a migration.
///
/// Steps map to DOL IR operations and cover all three storage backends:
/// - **SQL**: DDL operations via [`ir::Statement`] (CREATE, ALTER, DROP, INDEX, etc.)
/// - **KV**: Key-value namespace and key-pattern operations via [`KvMigrationOp`]
/// - **Storage**: Object storage bucket and prefix operations via [`StorageMigrationOp`]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MigrationStep {
    /// A DOL IR statement (SQL DDL, DML, indexes, grants, transactions, etc.).
    Sql(ir::Statement<'static>),

    /// A key-value specific migration operation.
    Kv(KvMigrationOp),

    /// An object storage migration operation.
    Storage(StorageMigrationOp),
}

impl MigrationStep {
    // -- SQL convenience constructors --

    /// Create a table from a [`DefineEntityIR`](ir::DefineEntityIR).
    pub fn define_entity(ir: ir::DefineEntityIR) -> Self {
        Self::Sql(ir::Statement::DefineEntity(Box::new(ir)))
    }

    /// Drop a table by name (with IF EXISTS).
    pub fn drop_entity(name: &str) -> Self {
        Self::Sql(ir::Statement::DropEntity(ir::DropEntityIR {
            target: ir::EntityRef {
                name: name.to_string(),
                namespace: None,
                alias: None,
            },
            if_exists: true,
            cascade: false,
        }))
    }

    /// Drop a table by name with CASCADE.
    pub fn drop_model_cascade(name: &str) -> Self {
        Self::Sql(ir::Statement::DropEntity(ir::DropEntityIR {
            target: ir::EntityRef {
                name: name.to_string(),
                namespace: None,
                alias: None,
            },
            if_exists: true,
            cascade: true,
        }))
    }

    /// Alter a table with the given actions.
    pub fn alter_entity(name: &str, actions: Vec<ir::AlterAction>) -> Self {
        Self::Sql(ir::Statement::AlterEntity(ir::AlterEntityIR {
            target: ir::EntityRef {
                name: name.to_string(),
                namespace: None,
                alias: None,
            },
            actions,
        }))
    }

    /// Create an index from a [`DefineIndexIR`](ir::DefineIndexIR).
    pub fn define_index(ir: ir::DefineIndexIR) -> Self {
        Self::Sql(ir::Statement::DefineIndex(ir))
    }

    /// Drop an index by name (with IF EXISTS).
    pub fn drop_index(name: &str) -> Self {
        Self::Sql(ir::Statement::DropIndex(ir::DropIndexIR {
            name: name.to_string(),
            if_exists: true,
            concurrently: false,
            cascade: false,
        }))
    }

    /// Create a custom type (e.g., enum).
    pub fn define_type(name: &str, variants: Vec<String>) -> Self {
        Self::Sql(ir::Statement::DefineType(ir::DefineTypeIR {
            name: name.to_string(),
            namespace: None,
            variants,
        }))
    }

    /// Drop a custom type by name.
    pub fn drop_type(name: &str) -> Self {
        Self::Sql(ir::Statement::DropType(ir::DropTypeIR {
            name: name.to_string(),
            if_exists: true,
        }))
    }

    /// Grant privileges.
    pub fn grant(ir: ir::GrantIR) -> Self {
        Self::Sql(ir::Statement::Grant(ir))
    }

    /// Revoke privileges.
    pub fn revoke(ir: ir::RevokeIR) -> Self {
        Self::Sql(ir::Statement::Revoke(ir))
    }

    /// Wrap any raw [`ir::Statement`].
    pub fn raw_statement(stmt: ir::Statement<'static>) -> Self {
        Self::Sql(stmt)
    }

    // -- KV convenience constructors --

    /// Create a KV namespace (prefix).
    pub fn create_kv_namespace(prefix: impl Into<String>) -> Self {
        Self::Kv(KvMigrationOp::CreateNamespace {
            prefix: prefix.into(),
        })
    }

    /// Drop a KV namespace (remove all keys with prefix).
    pub fn drop_kv_namespace(prefix: impl Into<String>) -> Self {
        Self::Kv(KvMigrationOp::DropNamespace {
            prefix: prefix.into(),
        })
    }

    /// Rename keys matching a pattern.
    pub fn rename_kv_keys(from_pattern: impl Into<String>, to_pattern: impl Into<String>) -> Self {
        Self::Kv(KvMigrationOp::RenameKeys {
            from_pattern: from_pattern.into(),
            to_pattern: to_pattern.into(),
        })
    }

    // -- Storage convenience constructors --

    /// Create an object storage bucket.
    pub fn create_bucket(name: impl Into<String>) -> Self {
        Self::Storage(StorageMigrationOp::CreateBucket {
            name: name.into(),
            region: None,
        })
    }

    /// Create an object storage bucket in a specific region.
    pub fn create_bucket_in_region(name: impl Into<String>, region: impl Into<String>) -> Self {
        Self::Storage(StorageMigrationOp::CreateBucket {
            name: name.into(),
            region: Some(region.into()),
        })
    }

    /// Delete an object storage bucket.
    pub fn delete_bucket(name: impl Into<String>) -> Self {
        Self::Storage(StorageMigrationOp::DeleteBucket { name: name.into() })
    }

    /// Returns the kind of this step for display/logging.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Sql(_) => "sql",
            Self::Kv(_) => "kv",
            Self::Storage(_) => "storage",
        }
    }
}

// ===========================================================================
// KV migration operations
// ===========================================================================

/// Key-value specific migration operations.
///
/// These operations model structural changes to key-value stores that have
/// no direct equivalent in the DOL IR statement system.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum KvMigrationOp {
    /// Create a new key namespace (prefix).
    ///
    /// Semantically marks that a prefix is now "in use" by the application.
    /// Implementations may create metadata keys, set up access rules, etc.
    CreateNamespace {
        /// The key prefix (e.g., `"users:"`, `"sessions:"`).
        prefix: String,
    },

    /// Drop a key namespace (remove all keys under a prefix).
    ///
    /// **Destructive**: All keys under this prefix will be removed.
    DropNamespace {
        /// The key prefix to drop.
        prefix: String,
    },

    /// Rename keys matching a pattern to a new pattern.
    ///
    /// Used for key schema evolution (e.g., renaming `user:{id}` to `usr:{id}`).
    RenameKeys {
        /// Source key pattern (e.g., `"user:*"`).
        from_pattern: String,
        /// Target key pattern (e.g., `"usr:*"`).
        to_pattern: String,
    },

    /// Set a TTL (time-to-live) on keys matching a pattern.
    SetTtl {
        /// Key pattern to apply TTL to.
        pattern: String,
        /// TTL in seconds.
        ttl_secs: u64,
    },

    /// Remove TTL from keys matching a pattern (make them persistent).
    RemoveTtl {
        /// Key pattern to remove TTL from.
        pattern: String,
    },

    /// Arbitrary custom operation for provider-specific migrations.
    Custom {
        /// Operation name for logging/tracking.
        operation: String,
        /// Key-value pairs of operation parameters.
        params: Vec<(String, String)>,
    },
}

impl fmt::Display for KvMigrationOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateNamespace { prefix } => write!(f, "CREATE NAMESPACE '{}'", prefix),
            Self::DropNamespace { prefix } => write!(f, "DROP NAMESPACE '{}'", prefix),
            Self::RenameKeys {
                from_pattern,
                to_pattern,
            } => write!(f, "RENAME KEYS '{}' -> '{}'", from_pattern, to_pattern),
            Self::SetTtl { pattern, ttl_secs } => {
                write!(f, "SET TTL {}s ON '{}'", ttl_secs, pattern)
            }
            Self::RemoveTtl { pattern } => write!(f, "REMOVE TTL ON '{}'", pattern),
            Self::Custom { operation, .. } => write!(f, "CUSTOM '{}'", operation),
        }
    }
}

// ===========================================================================
// Object storage migration operations
// ===========================================================================

/// Object storage specific migration operations.
///
/// These operations model structural changes to object/blob storage that have
/// no direct equivalent in the DOL IR statement system.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StorageMigrationOp {
    /// Create a new bucket / container.
    CreateBucket {
        /// Bucket name.
        name: String,
        /// Cloud region (optional).
        region: Option<String>,
    },

    /// Delete a bucket / container.
    ///
    /// **Destructive**: The bucket and all its contents will be removed.
    DeleteBucket {
        /// Bucket name.
        name: String,
    },

    /// Set a bucket-level policy (JSON policy document or similar).
    SetBucketPolicy {
        /// Bucket name.
        bucket: String,
        /// Policy document (format depends on the storage provider).
        policy: String,
    },

    /// Move all objects under one prefix to another prefix within the same bucket.
    MovePrefix {
        /// Bucket name.
        bucket: String,
        /// Source prefix.
        from_prefix: String,
        /// Destination prefix.
        to_prefix: String,
    },

    /// Set lifecycle rules on a bucket (e.g., auto-delete after N days).
    SetLifecycleRule {
        /// Bucket name.
        bucket: String,
        /// Rule identifier.
        rule_id: String,
        /// Object prefix this rule applies to.
        prefix: String,
        /// Number of days before expiration.
        expiration_days: u32,
    },

    /// Remove a lifecycle rule from a bucket.
    RemoveLifecycleRule {
        /// Bucket name.
        bucket: String,
        /// Rule identifier to remove.
        rule_id: String,
    },

    /// Arbitrary custom operation for provider-specific migrations.
    Custom {
        /// Operation name for logging/tracking.
        operation: String,
        /// Key-value pairs of operation parameters.
        params: Vec<(String, String)>,
    },
}

impl fmt::Display for StorageMigrationOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateBucket { name, region } => match region {
                Some(r) => write!(f, "CREATE BUCKET '{}' IN '{}'", name, r),
                None => write!(f, "CREATE BUCKET '{}'", name),
            },
            Self::DeleteBucket { name } => write!(f, "DELETE BUCKET '{}'", name),
            Self::SetBucketPolicy { bucket, .. } => {
                write!(f, "SET POLICY ON '{}'", bucket)
            }
            Self::MovePrefix {
                bucket,
                from_prefix,
                to_prefix,
            } => write!(
                f,
                "MOVE PREFIX '{}' -> '{}' IN '{}'",
                from_prefix, to_prefix, bucket
            ),
            Self::SetLifecycleRule {
                bucket,
                rule_id,
                expiration_days,
                ..
            } => write!(
                f,
                "SET LIFECYCLE '{}' ON '{}' (expire: {}d)",
                rule_id, bucket, expiration_days
            ),
            Self::RemoveLifecycleRule { bucket, rule_id } => {
                write!(f, "REMOVE LIFECYCLE '{}' FROM '{}'", rule_id, bucket)
            }
            Self::Custom { operation, .. } => write!(f, "CUSTOM '{}'", operation),
        }
    }
}

// ===========================================================================
// MigrationError
// ===========================================================================

/// Errors that can occur during migration operations.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MigrationError {
    /// Duplicate migration version detected.
    DuplicateVersion(String),

    /// A previously applied migration was modified (checksum mismatch).
    ChecksumMismatch {
        version: String,
        expected: String,
        actual: String,
    },

    /// The target version was not found in the migration list.
    TargetNotFound(String),

    /// A rendering error occurred.
    RenderError(String),

    /// A registry error occurred.
    RegistryError(String),

    /// Migration validation failed.
    ValidationError(String),
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateVersion(v) => write!(f, "duplicate migration version: {}", v),
            Self::ChecksumMismatch {
                version,
                expected,
                actual,
            } => write!(
                f,
                "checksum mismatch for migration '{}': expected {}, got {}",
                version, expected, actual
            ),
            Self::TargetNotFound(v) => write!(f, "target version '{}' not found", v),
            Self::RenderError(msg) => write!(f, "render error: {}", msg),
            Self::RegistryError(msg) => write!(f, "registry error: {}", msg),
            Self::ValidationError(msg) => write!(f, "validation error: {}", msg),
        }
    }
}

impl std::error::Error for MigrationError {}

#[cfg(test)]
pub(crate) mod test_helpers;

#[cfg(test)]
#[path = "async_tests.rs"]
mod async_tests;
