/// Migration configuration — unified settings for all storage backends.
///
/// This config lives at the top level of `DolConfig` (not nested under `sql`)
/// so that it applies to all backends uniformly.
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Backend enablement
// ---------------------------------------------------------------------------

/// Which backends should be included in migrations.
///
/// When `auto_detect` is true, the runner inspects registered migrations and
/// only produces output for backends that have steps (SQL, KV, Storage).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendFilter {
    /// Include SQL migration steps.
    #[serde(default = "default_true")]
    pub sql: bool,

    /// Include KV migration steps.
    #[serde(default = "default_true")]
    pub kv: bool,

    /// Include object storage migration steps.
    #[serde(default = "default_true")]
    pub storage: bool,
}

fn default_true() -> bool {
    true
}

impl Default for BackendFilter {
    fn default() -> Self {
        Self {
            sql: true,
            kv: true,
            storage: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Lock strategy (for safe concurrent migrations)
// ---------------------------------------------------------------------------

/// How to acquire a lock before running migrations.
///
/// Only relevant for SQL backends. KV and Storage migrations are typically
/// idempotent and do not require external locking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockStrategy {
    /// No locking — suitable for single-instance deployments.
    #[default]
    None,

    /// Use an advisory lock (database-level). The lock ID is derived from
    /// the `table_name` hash. Supported by PostgreSQL and MySQL.
    AdvisoryLock,

    /// Use a lock table row. A separate `_dol_migration_lock` table is used.
    LockTable,
}

// ---------------------------------------------------------------------------
// Unified migration config
// ---------------------------------------------------------------------------

/// Complete migration configuration.
///
/// This struct is designed to live at the top level of `DolConfig`:
///
/// ```toml
/// [migrations]
/// auto_run = true
/// table_name = "_dol_migrations"
///
/// [migrations.backends]
/// sql = true
/// kv = true
/// storage = true
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMigrationConfig {
    /// Run pending migrations automatically on startup.
    #[serde(default)]
    pub auto_run: bool,

    /// Name of the migration history table (SQL backend only).
    #[serde(default = "default_table_name")]
    pub table_name: String,

    /// Which backends to include when running migrations.
    #[serde(default)]
    pub backends: BackendFilter,

    /// Locking strategy for safe concurrent migrations.
    #[serde(default)]
    pub lock_strategy: LockStrategy,

    /// Validate checksums of previously applied migrations before running.
    #[serde(default = "default_true")]
    pub validate_checksums: bool,

    /// Abort on the first error instead of continuing with remaining migrations.
    #[serde(default = "default_true")]
    pub abort_on_error: bool,

    /// Dry-run mode — plan and render but do not execute.
    #[serde(default)]
    pub dry_run: bool,
}

fn default_table_name() -> String {
    "_dol_migrations".into()
}

impl Default for UnifiedMigrationConfig {
    fn default() -> Self {
        Self {
            auto_run: false,
            table_name: default_table_name(),
            backends: BackendFilter::default(),
            lock_strategy: LockStrategy::default(),
            validate_checksums: true,
            abort_on_error: true,
            dry_run: false,
        }
    }
}

#[cfg(test)]
#[path = "migration_tests.rs"]
mod tests;
