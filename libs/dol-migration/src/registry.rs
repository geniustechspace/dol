//! Migration registry — tracks which migrations have been applied.

use super::MigrationError;
use dol_core::builder::EntityBuilderExt;
use dol_core::model::{Entity, Field, FieldType};
use dol_sql::ext::Render;

// ===========================================================================
// Migration history model — the DOL model for tracking applied migrations
// ===========================================================================

/// The DOL model used to track applied migrations.
///
/// When using a SQL backend, the runner will create this table automatically
/// to persist migration history. For KV or in-memory registries, the model
/// is used as a schema reference.
///
/// Columns:
/// - `version` (VARCHAR(255), PK): Unique migration version identifier.
/// - `description` (TEXT): Human-readable description.
/// - `checksum` (TEXT): Hash of the migration steps for tamper detection.
/// - `applied_at` (TIMESTAMP): When the migration was applied.
/// - `execution_time_ms` (BIGINT): How long the migration took to run.
pub static MIGRATION_HISTORY: Entity = Entity::new(
    "_dol_migrations",
    &[
        Field::new("version", FieldType::Varchar(Some(255))).primary_key(),
        Field::new("description", FieldType::Text),
        Field::new("checksum", FieldType::Text),
        Field::new("applied_at", FieldType::Timestamp).default("CURRENT_TIMESTAMP"),
        Field::new("execution_time_ms", FieldType::BigInt).default("0"),
    ],
);

// ===========================================================================
// AppliedMigration — a record of a previously applied migration
// ===========================================================================

/// A record of a migration that has been applied.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AppliedMigration {
    /// The migration version.
    pub version: String,
    /// The migration description.
    pub description: String,
    /// The checksum of the migration steps at time of application.
    pub checksum: String,
    /// When the migration was applied (ISO 8601 string if available).
    pub applied_at: Option<String>,
    /// Execution time in milliseconds (if tracked).
    pub execution_time_ms: Option<u64>,
}

// ===========================================================================
// MigrationRegistry trait
// ===========================================================================

/// Trait for tracking which migrations have been applied.
///
/// Implementations persist migration history in different backends:
/// - SQL: A `_dol_migrations` table (see [`MIGRATION_HISTORY`])
/// - File: A JSON/TOML file on disk
/// - In-memory: For testing (see [`InMemoryRegistry`])
///
/// The DOL crate provides the trait and an in-memory implementation.
/// Backend-specific implementations (SQL, file) are expected to be provided
/// by the application or a driver crate, since DOL itself has no I/O.
pub trait MigrationRegistry {
    /// Get all previously applied migrations, ordered by version.
    fn applied(&self) -> Result<Vec<AppliedMigration>, MigrationError>;

    /// Record a migration as applied.
    fn mark_applied(
        &mut self,
        version: &str,
        description: &str,
        checksum: &str,
    ) -> Result<(), MigrationError>;

    /// Remove a migration from the applied list (on rollback).
    fn mark_reverted(&mut self, version: &str) -> Result<(), MigrationError>;
}

// ===========================================================================
// InMemoryRegistry — for testing
// ===========================================================================

/// An in-memory migration registry for testing.
///
/// ```rust
/// use dol_migration::{InMemoryRegistry, MigrationRegistry};
///
/// let mut registry = InMemoryRegistry::new();
/// registry.mark_applied("V001", "Create users", "abc123").unwrap();
///
/// let applied = registry.applied().unwrap();
/// assert_eq!(applied.len(), 1);
/// assert_eq!(applied[0].version, "V001");
///
/// registry.mark_reverted("V001").unwrap();
/// assert!(registry.applied().unwrap().is_empty());
/// ```
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InMemoryRegistry {
    applied: Vec<AppliedMigration>,
}

impl InMemoryRegistry {
    /// Create a new empty in-memory registry.
    pub fn new() -> Self {
        Self::default()
    }
}

impl MigrationRegistry for InMemoryRegistry {
    fn applied(&self) -> Result<Vec<AppliedMigration>, MigrationError> {
        let mut sorted = self.applied.clone();
        sorted.sort_by(|a, b| a.version.cmp(&b.version));
        Ok(sorted)
    }

    fn mark_applied(
        &mut self,
        version: &str,
        description: &str,
        checksum: &str,
    ) -> Result<(), MigrationError> {
        // Check for duplicates
        if self.applied.iter().any(|a| a.version == version) {
            return Err(MigrationError::DuplicateVersion(version.to_string()));
        }
        self.applied.push(AppliedMigration {
            version: version.to_string(),
            description: description.to_string(),
            checksum: checksum.to_string(),
            applied_at: None,
            execution_time_ms: None,
        });
        Ok(())
    }

    fn mark_reverted(&mut self, version: &str) -> Result<(), MigrationError> {
        self.applied.retain(|a| a.version != version);
        Ok(())
    }
}

// ===========================================================================
// SQL helpers for building a SQL-based registry
// ===========================================================================

/// Generate the CREATE TABLE SQL for the migration history table.
///
/// This helper allows applications to bootstrap the migration tracking table
/// in their SQL database.
///
/// ```rust
/// use dol_migration::registry;
/// use dol_sql::dialect::Dialect;
///
/// let sql = registry::create_history_table_sql(Some(&Dialect::postgres()));
/// assert!(sql.contains("CREATE TABLE IF NOT EXISTS _dol_migrations"));
/// ```
pub fn create_history_table_sql(dialect: Option<&dol_sql::dialect::Dialect>) -> String {
    MIGRATION_HISTORY
        .create()
        .if_not_exists()
        .render(dialect)
        .unwrap()
}

/// Generate an INSERT SQL for recording an applied migration.
///
/// Returns the SQL string with bind parameters for (version, description, checksum).
///
/// ```rust
/// use dol_migration::registry;
/// use dol_sql::dialect::Dialect;
///
/// let sql = registry::insert_applied_sql(Some(&Dialect::postgres()));
/// assert!(sql.contains("INSERT INTO _dol_migrations"));
/// assert!(sql.contains("$1")); // PostgreSQL params
/// ```
pub fn insert_applied_sql(dialect: Option<&dol_sql::dialect::Dialect>) -> String {
    MIGRATION_HISTORY
        .insert()
        .columns(&["version", "description", "checksum"])
        .render(dialect)
        .unwrap()
}

/// Generate a DELETE SQL for removing a migration record (on rollback).
///
/// Returns the SQL string with a bind parameter for the version.
///
/// ```rust
/// use dol_migration::registry;
/// use dol_sql::dialect::Dialect;
///
/// let sql = registry::delete_reverted_sql(Some(&Dialect::postgres()));
/// assert!(sql.contains("DELETE FROM _dol_migrations"));
/// assert!(sql.contains("$1")); // PostgreSQL params
/// ```
pub fn delete_reverted_sql(dialect: Option<&dol_sql::dialect::Dialect>) -> String {
    MIGRATION_HISTORY
        .remove()
        .where_eq("version")
        .render(dialect)
        .unwrap()
}

/// Generate a SELECT SQL for fetching all applied migrations.
///
/// ```rust
/// use dol_migration::registry;
/// use dol_sql::dialect::Dialect;
///
/// let sql = registry::select_applied_sql(Some(&Dialect::postgres()));
/// assert!(sql.contains("SELECT"));
/// assert!(sql.contains("_dol_migrations"));
/// ```
pub fn select_applied_sql(dialect: Option<&dol_sql::dialect::Dialect>) -> String {
    MIGRATION_HISTORY.get().render(dialect).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_registry_basic() {
        let mut reg = InMemoryRegistry::new();
        assert!(reg.applied().unwrap().is_empty());

        reg.mark_applied("V001", "First migration", "checksum1")
            .unwrap();
        reg.mark_applied("V002", "Second migration", "checksum2")
            .unwrap();

        let applied = reg.applied().unwrap();
        assert_eq!(applied.len(), 2);
        assert_eq!(applied[0].version, "V001");
        assert_eq!(applied[1].version, "V002");
    }

    #[test]
    fn in_memory_registry_revert() {
        let mut reg = InMemoryRegistry::new();
        reg.mark_applied("V001", "First", "c1").unwrap();
        reg.mark_applied("V002", "Second", "c2").unwrap();

        reg.mark_reverted("V001").unwrap();
        let applied = reg.applied().unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].version, "V002");
    }

    #[test]
    fn in_memory_registry_duplicate() {
        let mut reg = InMemoryRegistry::new();
        reg.mark_applied("V001", "First", "c1").unwrap();
        let result = reg.mark_applied("V001", "Duplicate", "c2");
        assert!(matches!(result, Err(MigrationError::DuplicateVersion(_))));
    }

    #[test]
    fn in_memory_registry_ordered() {
        let mut reg = InMemoryRegistry::new();
        // Insert out of order
        reg.mark_applied("V003", "Third", "c3").unwrap();
        reg.mark_applied("V001", "First", "c1").unwrap();
        reg.mark_applied("V002", "Second", "c2").unwrap();

        let applied = reg.applied().unwrap();
        assert_eq!(applied[0].version, "V001");
        assert_eq!(applied[1].version, "V002");
        assert_eq!(applied[2].version, "V003");
    }

    #[test]
    fn migration_history_entity() {
        assert_eq!(MIGRATION_HISTORY.name, "_dol_migrations");
        assert_eq!(MIGRATION_HISTORY.fields.len(), 5);
        assert!(MIGRATION_HISTORY.field("version").primary_key);
    }

    #[test]
    fn sql_helpers_postgres() {
        let pg = dol_sql::dialect::Dialect::postgres();

        let create_sql = create_history_table_sql(Some(&pg));
        assert!(create_sql.contains("CREATE TABLE IF NOT EXISTS _dol_migrations"));
        assert!(create_sql.contains("version VARCHAR(255) NOT NULL"));
        assert!(create_sql.contains("checksum TEXT NOT NULL"));

        let insert_sql = insert_applied_sql(Some(&pg));
        assert!(insert_sql.contains("INSERT INTO _dol_migrations"));
        assert!(insert_sql.contains("$1"));

        let delete_sql = delete_reverted_sql(Some(&pg));
        assert!(delete_sql.contains("DELETE FROM _dol_migrations"));
        assert!(delete_sql.contains("$1"));

        let select_sql = select_applied_sql(Some(&pg));
        assert!(select_sql.contains("SELECT"));
        assert!(select_sql.contains("_dol_migrations"));
    }

    #[test]
    fn sql_helpers_sqlite() {
        let create_sql = create_history_table_sql(None); // default is SQLite
        assert!(create_sql.contains("CREATE TABLE IF NOT EXISTS _dol_migrations"));

        let insert_sql = insert_applied_sql(None);
        assert!(insert_sql.contains("?"));

        let delete_sql = delete_reverted_sql(None);
        assert!(delete_sql.contains("?"));
    }
}
