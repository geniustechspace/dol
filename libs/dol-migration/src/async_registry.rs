//! Async migration registry — async-first alternative to [`MigrationRegistry`].
//!
//! This module provides [`AsyncMigrationRegistry`], the async counterpart of the
//! sync [`MigrationRegistry`](super::MigrationRegistry) trait. Use this trait
//! when your migration registry communicates with an external store (SQL database,
//! remote file, etc.) and you need non-blocking I/O.
//!
//! An in-memory implementation ([`InMemoryAsyncRegistry`]) is provided for testing.

use super::MigrationError;
use super::registry::AppliedMigration;

// ===========================================================================
// AsyncMigrationRegistry trait
// ===========================================================================

/// Async trait for tracking which migrations have been applied.
///
/// This is the async counterpart of [`MigrationRegistry`](super::MigrationRegistry).
/// Implement this trait to integrate with async database drivers or remote stores.
///
/// # Example
///
/// ```rust
/// use dol_migration::async_registry::{AsyncMigrationRegistry, InMemoryAsyncRegistry};
///
/// # async fn example() {
/// let mut registry = InMemoryAsyncRegistry::new();
/// registry.mark_applied("V001", "Create users", "abc123").await.unwrap();
///
/// let applied = registry.applied().await.unwrap();
/// assert_eq!(applied.len(), 1);
/// assert_eq!(applied[0].version, "V001");
///
/// registry.mark_reverted("V001").await.unwrap();
/// assert!(registry.applied().await.unwrap().is_empty());
/// # }
/// ```
pub trait AsyncMigrationRegistry: Send + Sync {
    /// Get all previously applied migrations, ordered by version.
    fn applied(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<AppliedMigration>, MigrationError>> + Send;

    /// Record a migration as applied.
    fn mark_applied(
        &mut self,
        version: &str,
        description: &str,
        checksum: &str,
    ) -> impl std::future::Future<Output = Result<(), MigrationError>> + Send;

    /// Remove a migration from the applied list (on rollback).
    fn mark_reverted(
        &mut self,
        version: &str,
    ) -> impl std::future::Future<Output = Result<(), MigrationError>> + Send;
}

// ===========================================================================
// InMemoryAsyncRegistry — for testing async code paths
// ===========================================================================

/// An in-memory async migration registry for testing.
///
/// This wraps the same logic as [`InMemoryRegistry`](super::InMemoryRegistry)
/// but exposes an async interface for use in `#[tokio::test]` or similar contexts.
///
/// ```rust
/// use dol_migration::async_registry::{AsyncMigrationRegistry, InMemoryAsyncRegistry};
///
/// # async fn example() {
/// let mut registry = InMemoryAsyncRegistry::new();
/// registry.mark_applied("V001", "First migration", "checksum1").await.unwrap();
///
/// let applied = registry.applied().await.unwrap();
/// assert_eq!(applied.len(), 1);
/// # }
/// ```
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InMemoryAsyncRegistry {
    applied: Vec<AppliedMigration>,
}

impl InMemoryAsyncRegistry {
    /// Create a new empty in-memory async registry.
    pub fn new() -> Self {
        Self::default()
    }
}

impl AsyncMigrationRegistry for InMemoryAsyncRegistry {
    async fn applied(&self) -> Result<Vec<AppliedMigration>, MigrationError> {
        let mut sorted = self.applied.clone();
        sorted.sort_by(|a, b| a.version.cmp(&b.version));
        Ok(sorted)
    }

    async fn mark_applied(
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

    async fn mark_reverted(&mut self, version: &str) -> Result<(), MigrationError> {
        self.applied.retain(|a| a.version != version);
        Ok(())
    }
}

// ===========================================================================
// Blanket impl: every sync MigrationRegistry is also an AsyncMigrationRegistry
// ===========================================================================

impl<T> AsyncMigrationRegistry for T
where
    T: super::MigrationRegistry + Send + Sync,
{
    async fn applied(&self) -> Result<Vec<AppliedMigration>, MigrationError> {
        super::MigrationRegistry::applied(self)
    }

    async fn mark_applied(
        &mut self,
        version: &str,
        description: &str,
        checksum: &str,
    ) -> Result<(), MigrationError> {
        super::MigrationRegistry::mark_applied(self, version, description, checksum)
    }

    async fn mark_reverted(&mut self, version: &str) -> Result<(), MigrationError> {
        super::MigrationRegistry::mark_reverted(self, version)
    }
}
