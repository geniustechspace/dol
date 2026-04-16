//! Async extensions for [`MigrationRunner`].
//!
//! Provides async versions of planning, validation, and status methods
//! that work with [`AsyncMigrationRegistry`](super::async_registry::AsyncMigrationRegistry).

use super::MigrationError;
use super::async_registry::AsyncMigrationRegistry;
use super::plan::{self, MigrationPlan, MigrationTarget};
use super::runner::{MigrationRunner, MigrationState, MigrationStatus};

impl MigrationRunner {
    /// Async version of [`plan`](Self::plan) — plan a migration to the given target.
    pub async fn plan_async<R: AsyncMigrationRegistry>(
        &self,
        registry: &R,
        target: MigrationTarget,
    ) -> Result<MigrationPlan, MigrationError> {
        self.validate()?;
        let applied = registry
            .applied()
            .await
            .map_err(|e| MigrationError::RegistryError(e.to_string()))?;
        plan::plan_to_target(self.migrations(), &applied, &target)
    }

    /// Async version of [`plan_forward`](Self::plan_forward).
    pub async fn plan_forward_async<R: AsyncMigrationRegistry>(
        &self,
        registry: &R,
    ) -> Result<MigrationPlan, MigrationError> {
        self.plan_async(registry, MigrationTarget::Latest).await
    }

    /// Async version of [`plan_rollback`](Self::plan_rollback).
    pub async fn plan_rollback_async<R: AsyncMigrationRegistry>(
        &self,
        registry: &R,
        n: usize,
    ) -> Result<MigrationPlan, MigrationError> {
        self.plan_async(registry, MigrationTarget::Rollback(n))
            .await
    }

    /// Async version of [`validate_checksums`](Self::validate_checksums).
    pub async fn validate_checksums_async<R: AsyncMigrationRegistry>(
        &self,
        registry: &R,
    ) -> Result<(), MigrationError> {
        let applied = registry
            .applied()
            .await
            .map_err(|e| MigrationError::RegistryError(e.to_string()))?;

        for applied_migration in &applied {
            if let Some(migration) = self
                .migrations()
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

    /// Async version of [`status`](Self::status).
    pub async fn status_async<R: AsyncMigrationRegistry>(
        &self,
        registry: &R,
    ) -> Result<Vec<MigrationStatus>, MigrationError> {
        let applied = registry
            .applied()
            .await
            .map_err(|e| MigrationError::RegistryError(e.to_string()))?;
        let applied_map: std::collections::HashMap<&str, &super::registry::AppliedMigration> =
            applied.iter().map(|a| (a.version.as_str(), a)).collect();

        Ok(self
            .migrations()
            .iter()
            .map(|m| {
                let version = m.version().to_string();
                let description = m.description().to_string();
                match applied_map.get(m.version()) {
                    Some(am) => MigrationStatus {
                        version,
                        description,
                        state: MigrationState::Applied,
                        applied_at: am.applied_at.clone(),
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
}
