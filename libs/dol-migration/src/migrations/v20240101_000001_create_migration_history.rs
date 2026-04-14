//! Migration: Create the `_dol_migrations` history table.
//!
//! This is the foundational SQL migration that creates the table DOL uses
//! to track which migrations have been applied. It must run before any
//! other migration can be recorded.

use crate::{Migration, MigrationStep};
use dol_core::builder::DefineModelBuilder;
use dol_core::ir::ModelRef;
use dol_core::ir::definition::{DefineIndexIR, FieldDef};
use dol_core::model::FieldType;

/// Creates the `_dol_migrations` table and an index on the `applied_at` column.
///
/// **Forward (up)**:
/// - `CREATE TABLE IF NOT EXISTS _dol_migrations (version, description, checksum, applied_at, execution_time_ms)`
/// - `CREATE INDEX IF NOT EXISTS idx_dol_migrations_applied_at ON _dol_migrations (applied_at)`
///
/// **Backward (down)**:
/// - `DROP INDEX IF EXISTS idx_dol_migrations_applied_at`
/// - `DROP TABLE IF EXISTS _dol_migrations`
pub struct CreateMigrationHistory;

impl Migration for CreateMigrationHistory {
    fn version(&self) -> &str {
        "20240101_000001"
    }

    fn description(&self) -> &str {
        "Create _dol_migrations history table"
    }

    fn up(&self) -> Vec<MigrationStep> {
        vec![
            // Create the migration tracking table
            MigrationStep::define_model(
                DefineModelBuilder::new("_dol_migrations")
                    .field(FieldDef::new("version", FieldType::Text).primary_key())
                    .field(FieldDef::new("description", FieldType::Text))
                    .field(FieldDef::new("checksum", FieldType::Text))
                    .field(FieldDef::new("applied_at", FieldType::Timestamp).default("NOW()"))
                    .field(
                        FieldDef::new("execution_time_ms", FieldType::BigInt)
                            .default("0"),
                    )
                    .if_not_exists()
                    .build(),
            ),
            // Index on applied_at for chronological queries
            MigrationStep::define_index(DefineIndexIR {
                name: "idx_dol_migrations_applied_at".to_string(),
                target: ModelRef {
                    name: "_dol_migrations".to_string(),
                    namespace: None,
                    alias: None,
                },
                columns: vec!["applied_at".to_string()],
                unique: false,
                if_not_exists: true,
                concurrently: false,
                method: None,
                where_clause: None,
            }),
        ]
    }

    fn down(&self) -> Vec<MigrationStep> {
        vec![
            MigrationStep::drop_index("idx_dol_migrations_applied_at"),
            MigrationStep::drop_model("_dol_migrations"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dol_sql::dialect::Dialect;

    #[test]
    fn migration_version_and_description() {
        let m = CreateMigrationHistory;
        assert_eq!(m.version(), "20240101_000001");
        assert!(!m.description().is_empty());
    }

    #[test]
    fn up_creates_table_and_index() {
        let steps = CreateMigrationHistory.up();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].kind(), "sql");
        assert_eq!(steps[1].kind(), "sql");
    }

    #[test]
    fn down_drops_index_and_table() {
        let steps = CreateMigrationHistory.down();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].kind(), "sql");
        assert_eq!(steps[1].kind(), "sql");
    }

    #[test]
    fn renders_to_postgres() {
        let pg = Dialect::postgres();
        let runner = crate::MigrationRunner::new().register(CreateMigrationHistory);
        let registry = crate::InMemoryRegistry::new();
        let plan = runner.plan_forward(&registry).unwrap();
        let rendered = runner.render_plan(&plan, Some(&pg));

        assert_eq!(rendered.len(), 1);
        let sql0 = rendered[0].steps[0].sql().unwrap();
        assert!(sql0.contains("CREATE TABLE IF NOT EXISTS"));
        assert!(sql0.contains("_dol_migrations"));

        let sql1 = rendered[0].steps[1].sql().unwrap();
        assert!(sql1.contains("CREATE INDEX"));
        assert!(sql1.contains("idx_dol_migrations_applied_at"));
    }

    #[test]
    fn renders_to_mysql() {
        let mysql = Dialect::mysql();
        let runner = crate::MigrationRunner::new().register(CreateMigrationHistory);
        let registry = crate::InMemoryRegistry::new();
        let plan = runner.plan_forward(&registry).unwrap();
        let rendered = runner.render_plan(&plan, Some(&mysql));

        assert_eq!(rendered.len(), 1);
        let sql0 = rendered[0].steps[0].sql().unwrap();
        assert!(sql0.contains("CREATE TABLE IF NOT EXISTS"));
        assert!(sql0.contains("_dol_migrations"));
    }

    #[test]
    fn renders_to_sqlite() {
        let sqlite = Dialect::sqlite();
        let runner = crate::MigrationRunner::new().register(CreateMigrationHistory);
        let registry = crate::InMemoryRegistry::new();
        let plan = runner.plan_forward(&registry).unwrap();
        let rendered = runner.render_plan(&plan, Some(&sqlite));

        assert_eq!(rendered.len(), 1);
        let sql0 = rendered[0].steps[0].sql().unwrap();
        assert!(sql0.contains("CREATE TABLE IF NOT EXISTS"));
        assert!(sql0.contains("_dol_migrations"));
    }
}
