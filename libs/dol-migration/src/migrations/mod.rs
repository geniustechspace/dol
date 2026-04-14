//! Built-in migration files for bootstrapping DOL infrastructure.
//!
//! These migrations set up the internal tables, namespaces, and buckets that DOL
//! needs to operate. They cover all three backend types:
//!
//! - **SQL**: Creates the `_dol_migrations` history table (and an index on it)
//! - **KV**: Creates the `dol:migrations:` namespace for tracking migration state
//! - **Storage**: Creates the `dol-migrations` bucket for migration artifacts
//!
//! # Usage
//!
//! Register the built-in migrations with a [`MigrationRunner`](crate::MigrationRunner)
//! before any application migrations:
//!
//! ```rust
//! use dol_migration::{MigrationRunner, migrations};
//!
//! let runner = MigrationRunner::new()
//!     // DOL infrastructure (runs first)
//!     .register(migrations::CreateMigrationHistory)
//!     .register(migrations::CreateKvMigrationNamespace)
//!     .register(migrations::CreateStorageMigrationBucket)
//!     // Application migrations (after infrastructure)
//!     // .register(my_app::migrations::CreateUsersTable)
//!     ;
//! ```
//!
//! If your application only uses a subset of backends (e.g., SQL only), you can
//! register only the relevant migrations. The migration runner's filtered
//! rendering will skip steps for disabled backends.

mod v20240101_000001_create_migration_history;
mod v20240101_000002_create_kv_migration_namespace;
mod v20240101_000003_create_storage_migration_bucket;

pub use v20240101_000001_create_migration_history::CreateMigrationHistory;
pub use v20240101_000002_create_kv_migration_namespace::CreateKvMigrationNamespace;
pub use v20240101_000003_create_storage_migration_bucket::CreateStorageMigrationBucket;

/// Register all built-in DOL infrastructure migrations on a runner.
///
/// This is a convenience method that registers the three bootstrap migrations
/// in the correct order.
///
/// ```rust
/// use dol_migration::{MigrationRunner, migrations};
///
/// let runner = migrations::register_builtins(MigrationRunner::new());
/// assert_eq!(runner.count(), 3);
/// ```
pub fn register_builtins(runner: crate::MigrationRunner) -> crate::MigrationRunner {
    runner
        .register(CreateMigrationHistory)
        .register(CreateKvMigrationNamespace)
        .register(CreateStorageMigrationBucket)
}
