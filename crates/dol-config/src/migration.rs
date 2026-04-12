/// Migration configuration — unified settings for all storage backends.
///
/// This config lives at the top level of `DolConfig` (not nested under `sql`)
/// so that it applies to all backends uniformly.
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Backend enablement
// ---------------------------------------------------------------------------

/// Which backends should be included in migrations.
///
/// When `auto_detect` is true, the runner inspects registered migrations and
/// only produces output for backends that have steps (SQL, KV, Storage).
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Default, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
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
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let cfg = UnifiedMigrationConfig::default();
        assert!(!cfg.auto_run);
        assert_eq!(cfg.table_name, "_dol_migrations");
        assert!(cfg.backends.sql);
        assert!(cfg.backends.kv);
        assert!(cfg.backends.storage);
        assert!(cfg.validate_checksums);
        assert!(cfg.abort_on_error);
        assert!(!cfg.dry_run);
    }

    #[test]
    fn deserialize_from_toml() {
        let toml_str = r#"
            auto_run = true
            table_name = "schema_versions"
            dry_run = true
            lock_strategy = "advisory_lock"

            [backends]
            sql = true
            kv = false
            storage = false
        "#;
        let cfg: UnifiedMigrationConfig = toml_crate::from_str(toml_str).unwrap();
        assert!(cfg.auto_run);
        assert_eq!(cfg.table_name, "schema_versions");
        assert!(cfg.backends.sql);
        assert!(!cfg.backends.kv);
        assert!(!cfg.backends.storage);
        assert!(cfg.dry_run);
        assert!(matches!(cfg.lock_strategy, LockStrategy::AdvisoryLock));
    }

    #[test]
    fn deserialize_minimal() {
        let toml_str = "auto_run = true";
        let cfg: UnifiedMigrationConfig = toml_crate::from_str(toml_str).unwrap();
        assert!(cfg.auto_run);
        assert_eq!(cfg.table_name, "_dol_migrations");
    }

    #[test]
    fn lock_strategies() {
        let cases = [
            (r#"lock_strategy = "none""#, "none"),
            (r#"lock_strategy = "advisory_lock""#, "advisory_lock"),
            (r#"lock_strategy = "lock_table""#, "lock_table"),
        ];
        for (toml_str, _label) in cases {
            let cfg: UnifiedMigrationConfig = toml_crate::from_str(toml_str).unwrap();
            // Just ensure it deserializes without error
            let _ = cfg.lock_strategy;
        }
    }
}
