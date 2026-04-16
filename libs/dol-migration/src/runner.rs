//! Migration runner — orchestrates planning, rendering, and validation.

use super::plan::{self, MigrationDirection, MigrationPlan, MigrationTarget, PlannedStep};
use super::registry::{AppliedMigration, MigrationRegistry};
use super::{KvMigrationOp, Migration, MigrationError, MigrationStep, StorageMigrationOp};
use dol_core::ir::BackendError;
use dol_core::ir::Statement;
use dol_sql::dialect::{self, Dialect};
use dol_sql::render;

// ===========================================================================
// RenderedStep — the output of rendering a migration step
// ===========================================================================

/// A rendered migration step, ready for execution.
///
/// Each step renders to one of three output types:
/// - **SQL**: A SQL string + parameter count
/// - **KV**: A key-value operation descriptor
/// - **Storage**: An object storage operation descriptor
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RenderedStep {
    /// A rendered SQL statement.
    Sql {
        /// The SQL string.
        sql: String,
        /// Number of bind parameters.
        param_count: usize,
    },
    /// A rendered KV migration operation.
    Kv {
        /// Human-readable description of the operation.
        description: String,
        /// The original operation (for execution by a KV driver).
        operation: KvMigrationOp,
    },
    /// A rendered object storage migration operation.
    Storage {
        /// Human-readable description of the operation.
        description: String,
        /// The original operation (for execution by a storage driver).
        operation: StorageMigrationOp,
    },
}

impl RenderedStep {
    /// Returns the SQL string if this is a SQL step.
    pub fn sql(&self) -> Option<&str> {
        match self {
            Self::Sql { sql, .. } => Some(sql),
            _ => None,
        }
    }

    /// Returns the parameter count if this is a SQL step.
    pub fn param_count(&self) -> Option<usize> {
        match self {
            Self::Sql { param_count, .. } => Some(*param_count),
            _ => None,
        }
    }

    /// Returns a human-readable description of this step.
    pub fn describe(&self) -> String {
        match self {
            Self::Sql { sql, .. } => {
                // First line or first 120 chars
                let first_line = sql.lines().next().unwrap_or(sql);
                if first_line.len() > 120 {
                    format!("{}...", &first_line[..120])
                } else {
                    first_line.to_string()
                }
            }
            Self::Kv { description, .. } => description.clone(),
            Self::Storage { description, .. } => description.clone(),
        }
    }
}

// ===========================================================================
// Rendered migration — all rendered steps for a single planned migration
// ===========================================================================

/// All rendered steps for a single planned migration, annotated with metadata.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RenderedMigration {
    /// The migration version.
    pub version: String,
    /// The migration description.
    pub description: String,
    /// Direction (forward or backward).
    pub direction: MigrationDirection,
    /// The rendered steps.
    pub steps: Vec<RenderedStep>,
}

// ===========================================================================
// MigrationRunner
// ===========================================================================

/// Orchestrates migration planning, rendering, and validation.
///
/// The runner is the main entry point for working with migrations. It holds
/// the list of registered migrations and provides methods to:
/// - **Plan**: Determine which migrations to apply/revert.
/// - **Render**: Convert planned steps to SQL/KV/Storage output (dry-run).
/// - **Validate**: Check for duplicate versions and checksum mismatches.
///
/// # Example
///
/// ```rust
/// use dol_migration::{
///     Migration, MigrationStep, MigrationRunner, MigrationTarget,
///     InMemoryRegistry, MigrationRegistry,
/// };
/// use dol_core::ir::definition::FieldDef;
/// use dol_core::model::FieldType;
///
/// struct CreateUsers;
/// impl Migration for CreateUsers {
///     fn version(&self) -> &str { "V001" }
///     fn description(&self) -> &str { "Create users table" }
///     fn up(&self) -> Vec<MigrationStep> {
///         vec![MigrationStep::define_entity(
///             dol_core::builder::definition::DefineEntityBuilder::new("users")
///                 .field(FieldDef::new("id", FieldType::Uuid).primary_key())
///                 .field(FieldDef::new("email", FieldType::Text).unique())
///                 .if_not_exists()
///                 .build()
///         )]
///     }
///     fn down(&self) -> Vec<MigrationStep> {
///         vec![MigrationStep::drop_entity("users")]
///     }
/// }
///
/// let runner = MigrationRunner::new().register(CreateUsers);
/// let registry = InMemoryRegistry::new();
///
/// // Plan
/// let plan = runner.plan(&registry, MigrationTarget::Latest).unwrap();
/// assert_eq!(plan.len(), 1);
///
/// // Render (dry-run)
/// let rendered = runner.render_plan(&plan, None);
/// assert_eq!(rendered.len(), 1);
/// ```
pub struct MigrationRunner {
    migrations: Vec<Box<dyn Migration>>,
}

impl MigrationRunner {
    /// Create a new runner with no migrations registered.
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    /// Register a migration. Migrations must be added in version order.
    pub fn register<M: Migration + 'static>(mut self, migration: M) -> Self {
        self.migrations.push(Box::new(migration));
        self
    }

    /// Register multiple migrations from an iterator.
    pub fn add_all(mut self, migrations: Vec<Box<dyn Migration>>) -> Self {
        self.migrations.extend(migrations);
        self
    }

    /// Returns the registered migrations.
    pub fn migrations(&self) -> &[Box<dyn Migration>] {
        &self.migrations
    }

    /// Returns the number of registered migrations.
    pub fn count(&self) -> usize {
        self.migrations.len()
    }

    // -- Validation --

    /// Validate that all migrations have unique versions.
    pub fn validate(&self) -> Result<(), MigrationError> {
        let mut seen = std::collections::HashSet::new();
        for m in &self.migrations {
            if !seen.insert(m.version()) {
                return Err(MigrationError::DuplicateVersion(m.version().to_string()));
            }
        }
        Ok(())
    }

    /// Validate checksums of applied migrations against current definitions.
    ///
    /// Returns an error if any applied migration has been modified since it
    /// was applied (checksum mismatch). This prevents silent schema drift.
    pub fn validate_checksums(
        &self,
        registry: &dyn MigrationRegistry,
    ) -> Result<(), MigrationError> {
        let applied = registry
            .applied()
            .map_err(|e| MigrationError::RegistryError(e.to_string()))?;

        for applied_migration in &applied {
            if let Some(migration) = self
                .migrations
                .iter()
                .find(|m| m.version() == applied_migration.version)
            {
                let current_checksum = plan::compute_checksum(&migration.up());
                if !applied_migration.checksum.is_empty()
                    && applied_migration.checksum != current_checksum
                {
                    return Err(MigrationError::ChecksumMismatch {
                        version: applied_migration.version.clone(),
                        expected: applied_migration.checksum.clone(),
                        actual: current_checksum,
                    });
                }
            }
        }

        Ok(())
    }

    // -- Planning --

    /// Plan a migration to the given target.
    pub fn plan(
        &self,
        registry: &dyn MigrationRegistry,
        target: MigrationTarget,
    ) -> Result<MigrationPlan, MigrationError> {
        self.validate()?;
        let applied = registry
            .applied()
            .map_err(|e| MigrationError::RegistryError(e.to_string()))?;
        plan::plan_to_target(&self.migrations, &applied, &target)
    }

    /// Convenience: plan forward to the latest version.
    pub fn plan_forward(
        &self,
        registry: &dyn MigrationRegistry,
    ) -> Result<MigrationPlan, MigrationError> {
        self.plan(registry, MigrationTarget::Latest)
    }

    /// Convenience: plan to roll back the last N migrations.
    pub fn plan_rollback(
        &self,
        registry: &dyn MigrationRegistry,
        n: usize,
    ) -> Result<MigrationPlan, MigrationError> {
        self.plan(registry, MigrationTarget::Rollback(n))
    }

    // -- Rendering --

    /// Render a plan to SQL/KV/Storage output.
    ///
    /// This is a dry-run mode — it produces the output without executing
    /// anything. Pass `None` for the dialect to use the global default (SQLite).
    pub fn render_plan(
        &self,
        plan: &MigrationPlan,
        dialect: Option<&Dialect>,
    ) -> Vec<RenderedMigration> {
        plan.steps()
            .iter()
            .map(|planned| self.render_planned_step(planned, dialect))
            .collect()
    }

    /// Render a single planned step.
    fn render_planned_step(
        &self,
        planned: &PlannedStep,
        dialect: Option<&Dialect>,
    ) -> RenderedMigration {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let rendered_steps: Vec<RenderedStep> = planned
            .steps
            .iter()
            .map(|step| render_step(step, dialect))
            .collect();

        RenderedMigration {
            version: planned.version.clone(),
            description: planned.description.clone(),
            direction: planned.direction,
            steps: rendered_steps,
        }
    }

    // -- Status --

    /// Get the status of all migrations (applied or pending).
    pub fn status(
        &self,
        registry: &dyn MigrationRegistry,
    ) -> Result<Vec<MigrationStatus>, MigrationError> {
        let applied = registry
            .applied()
            .map_err(|e| MigrationError::RegistryError(e.to_string()))?;
        let applied_map: std::collections::HashMap<&str, &AppliedMigration> =
            applied.iter().map(|a| (a.version.as_str(), a)).collect();

        Ok(self
            .migrations
            .iter()
            .map(|m| {
                let version = m.version().to_string();
                let description = m.description().to_string();
                match applied_map.get(m.version()) {
                    Some(applied) => MigrationStatus {
                        version,
                        description,
                        state: MigrationState::Applied,
                        applied_at: applied.applied_at.clone(),
                    },
                    None => MigrationStatus {
                        version,
                        description,
                        state: MigrationState::Pending,
                        applied_at: None,
                    },
                }
            })
            .collect())
    }
    // -- Config-integrated rendering --

    /// Render a plan, filtering steps based on a backend filter.
    ///
    /// Steps that don't match the enabled backends are excluded from the
    /// rendered output. This allows the config to control which backends
    /// receive migration operations.
    pub fn render_plan_filtered(
        &self,
        plan: &MigrationPlan,
        dialect: Option<&Dialect>,
        sql_enabled: bool,
        kv_enabled: bool,
        storage_enabled: bool,
    ) -> Vec<RenderedMigration> {
        plan.steps()
            .iter()
            .map(|planned| {
                self.render_planned_step_filtered(
                    planned,
                    dialect,
                    sql_enabled,
                    kv_enabled,
                    storage_enabled,
                )
            })
            .filter(|rm| !rm.steps.is_empty())
            .collect()
    }

    /// Render a single planned step with backend filtering.
    fn render_planned_step_filtered(
        &self,
        planned: &PlannedStep,
        dialect: Option<&Dialect>,
        sql_enabled: bool,
        kv_enabled: bool,
        storage_enabled: bool,
    ) -> RenderedMigration {
        let dialect = dialect.unwrap_or_else(|| dialect::default_dialect());
        let rendered_steps: Vec<RenderedStep> = planned
            .steps
            .iter()
            .filter(|step| match step {
                MigrationStep::Sql(_) => sql_enabled,
                MigrationStep::Kv(_) => kv_enabled,
                MigrationStep::Storage(_) => storage_enabled,
            })
            .map(|step| render_step(step, dialect))
            .collect();

        RenderedMigration {
            version: planned.version.clone(),
            description: planned.description.clone(),
            direction: planned.direction,
            steps: rendered_steps,
        }
    }
}

/// Config-integrated migration runner methods.
///
/// These methods are available when the `config` feature is enabled and
/// allow the runner to work directly with `DolConfig` settings.
#[cfg(feature = "config")]
impl MigrationRunner {
    /// Render a plan using settings from a `DolConfig`.
    ///
    /// Resolves the SQL dialect from config and applies the backend filter.
    pub fn render_plan_with_config(
        &self,
        plan: &MigrationPlan,
        config: &dol_config::DolConfig,
    ) -> Vec<RenderedMigration> {
        let dialect = config.sql.as_ref().map(|sql| sql.dialect.resolve());
        let dialect_ref = dialect.as_ref();
        let backends = &config.migrations.backends;

        self.render_plan_filtered(
            plan,
            dialect_ref,
            backends.sql,
            backends.kv,
            backends.storage,
        )
    }

    /// Check if auto-run is enabled in config.
    pub fn should_auto_run(config: &dol_config::DolConfig) -> bool {
        config.migrations.auto_run
    }

    /// Check if checksums should be validated.
    pub fn should_validate_checksums(config: &dol_config::DolConfig) -> bool {
        config.migrations.validate_checksums
    }

    /// Check if dry-run mode is enabled.
    pub fn is_dry_run(config: &dol_config::DolConfig) -> bool {
        config.migrations.dry_run
    }
}

impl Default for MigrationRunner {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// MigrationStatus — for reporting
// ===========================================================================

/// Status of a single migration.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MigrationStatus {
    /// The migration version.
    pub version: String,
    /// The migration description.
    pub description: String,
    /// Whether the migration has been applied or is pending.
    pub state: MigrationState,
    /// When the migration was applied (if applied).
    pub applied_at: Option<String>,
}

/// Whether a migration has been applied or is pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MigrationState {
    /// The migration has been applied.
    Applied,
    /// The migration is pending (not yet applied).
    Pending,
}

// ===========================================================================
// Step rendering
// ===========================================================================

/// Render a single [`MigrationStep`] to a [`RenderedStep`].
fn render_step(step: &MigrationStep, dialect: &Dialect) -> RenderedStep {
    match step {
        MigrationStep::Sql(stmt) => render_sql_step(stmt, dialect),
        MigrationStep::Kv(op) => RenderedStep::Kv {
            description: op.to_string(),
            operation: op.clone(),
        },
        MigrationStep::Storage(op) => RenderedStep::Storage {
            description: op.to_string(),
            operation: op.clone(),
        },
    }
}

/// Render a SQL statement to a [`RenderedStep`].
fn render_sql_step(stmt: &Statement, dialect: &Dialect) -> RenderedStep {
    let result: Result<dol_core::ir::SqlOutput, BackendError> = match stmt {
        Statement::DefineEntity(ir) => render::render_define_entity_ir(ir, dialect),
        Statement::AlterEntity(ir) => render::render_alter_entity_ir(ir, dialect),
        Statement::DropEntity(ir) => render::render_drop_entity_ir(ir, dialect),
        Statement::DefineIndex(ir) => render::render_define_index_ir(ir, dialect),
        Statement::DropIndex(ir) => render::render_drop_index_ir(ir, dialect),
        Statement::DefineType(ir) => render::render_define_type_ir(ir, dialect),
        Statement::DropType(ir) => render::render_drop_type_ir(ir, dialect),
        Statement::DefinePolicy(ir) => render::render_define_policy_ir(ir, dialect),
        Statement::Grant(ir) => render::render_grant_ir(ir),
        Statement::Revoke(ir) => render::render_revoke_ir(ir),
        Statement::Transaction(ir) => render::render_transaction_ir(ir, dialect),
        Statement::Insert(ir) => render::render_insert_ir(ir, dialect),
        Statement::InsertSelect(ir) => render::render_insert_select_ir(ir, dialect),
        Statement::Update(ir) => render::render_update_ir(ir, dialect),
        Statement::Remove(ir) => render::render_remove_ir(ir, dialect),
        Statement::Upsert(ir) => render::render_upsert_ir(ir, dialect),
        Statement::Query(ir) => render::render_query_ir(ir, dialect),
        Statement::Compound(ir) => render::render_compound_query_ir(ir, dialect),
        _ => Err(BackendError::Unsupported(format!(
            "unsupported statement in migration: {:?}",
            std::mem::discriminant(stmt)
        ))),
    };

    match result {
        Ok(output) => RenderedStep::Sql {
            sql: output.sql,
            param_count: output.param_count,
        },
        Err(e) => RenderedStep::Sql {
            sql: format!("-- RENDER ERROR: {}", e),
            param_count: 0,
        },
    }
}

#[cfg(test)]
#[path = "runner_tests.rs"]
mod tests;

// ===========================================================================
// Config-integrated tests (feature = "config")
// ===========================================================================

#[cfg(all(test, feature = "config"))]
mod config_tests {
    use super::*;
    use crate::InMemoryRegistry;
    use crate::test_helpers::{CreateUsersTable, SetupKvNamespaces, SetupStorageBuckets};
    use dol_config::DolConfig;

    #[test]
    fn render_plan_with_config_default() {
        let runner = MigrationRunner::new()
            .register(CreateUsersTable)
            .register(SetupKvNamespaces)
            .register(SetupStorageBuckets);

        let config: DolConfig = toml_crate::from_str("").unwrap();
        let registry = InMemoryRegistry::new();
        let plan = runner.plan_forward(&registry).unwrap();

        // Default config enables all backends
        assert!(config.migrations.backends.sql);
        assert!(config.migrations.backends.kv);
        assert!(config.migrations.backends.storage);
        let rendered = runner.render_plan_with_config(&plan, &config);
        assert_eq!(rendered.len(), 3);
    }

    #[test]
    fn render_plan_with_config_filtered() {
        let runner = MigrationRunner::new()
            .register(CreateUsersTable)
            .register(SetupKvNamespaces)
            .register(SetupStorageBuckets);

        let config: DolConfig = toml_crate::from_str(
            r#"
            [migrations]
            auto_run = true

            [migrations.backends]
            sql = true
            kv = false
            storage = false
        "#,
        )
        .unwrap();

        let registry = InMemoryRegistry::new();
        let plan = runner.plan_forward(&registry).unwrap();

        let rendered = runner.render_plan_with_config(&plan, &config);
        // Only SQL steps are enabled
        assert_eq!(rendered.len(), 1);
        assert!(rendered[0].steps[0].sql().is_some());
    }

    #[test]
    fn render_plan_with_config_dialect() {
        let runner = MigrationRunner::new().register(CreateUsersTable);

        let config: DolConfig = toml_crate::from_str(
            r#"
            [sql]
            dialect = "postgresql"
            [sql.primary]
            url = "postgres://localhost/test"
        "#,
        )
        .unwrap();

        let registry = InMemoryRegistry::new();
        let plan = runner.plan_forward(&registry).unwrap();

        let rendered = runner.render_plan_with_config(&plan, &config);
        let sql = rendered[0].steps[0].sql().unwrap();
        // PostgreSQL renders UUID type
        assert!(sql.contains("UUID"));
    }

    #[test]
    fn config_auto_run_detection() {
        let config: DolConfig = toml_crate::from_str("[migrations]\nauto_run = true").unwrap();
        assert!(MigrationRunner::should_auto_run(&config));

        let config: DolConfig = toml_crate::from_str("").unwrap();
        assert!(!MigrationRunner::should_auto_run(&config));
    }

    #[test]
    fn config_dry_run_detection() {
        let config: DolConfig = toml_crate::from_str("[migrations]\ndry_run = true").unwrap();
        assert!(MigrationRunner::is_dry_run(&config));
    }

    #[test]
    fn config_checksum_validation_detection() {
        let config: DolConfig = toml_crate::from_str("").unwrap();
        assert!(MigrationRunner::should_validate_checksums(&config));

        let config: DolConfig =
            toml_crate::from_str("[migrations]\nvalidate_checksums = false").unwrap();
        assert!(!MigrationRunner::should_validate_checksums(&config));
    }
}
