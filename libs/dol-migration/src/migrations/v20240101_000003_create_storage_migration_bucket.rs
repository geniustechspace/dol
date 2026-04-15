//! Migration: Create the `dol-migrations` object storage bucket.
//!
//! This migration sets up the object storage bucket that DOL can use to store
//! migration artifacts (snapshots, lock files, etc.) in storage backends
//! (S3, GCS, Azure Blob, local filesystem, etc.).

use crate::{Migration, MigrationStep};

/// Creates the `dol-migrations` bucket for migration artifacts.
///
/// **Forward (up)**:
/// - Creates the `dol-migrations` storage bucket
///
/// **Backward (down)**:
/// - Deletes the `dol-migrations` storage bucket
pub struct CreateStorageMigrationBucket;

impl Migration for CreateStorageMigrationBucket {
    fn version(&self) -> &str {
        "20240101_000003"
    }

    fn description(&self) -> &str {
        "Create dol-migrations storage bucket"
    }

    fn up(&self) -> Vec<MigrationStep> {
        vec![MigrationStep::create_bucket("dol-migrations")]
    }

    fn down(&self) -> Vec<MigrationStep> {
        vec![MigrationStep::delete_bucket("dol-migrations")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_version_and_description() {
        let m = CreateStorageMigrationBucket;
        assert_eq!(m.version(), "20240101_000003");
        assert!(!m.description().is_empty());
    }

    #[test]
    fn up_creates_bucket() {
        let steps = CreateStorageMigrationBucket.up();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].kind(), "storage");
    }

    #[test]
    fn down_deletes_bucket() {
        let steps = CreateStorageMigrationBucket.down();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].kind(), "storage");
    }

    #[test]
    fn renders_storage_operations() {
        let runner = crate::MigrationRunner::new().register(CreateStorageMigrationBucket);
        let registry = crate::InMemoryRegistry::new();
        let plan = runner.plan_forward(&registry).unwrap();
        let rendered = runner.render_plan(&plan, None);

        assert_eq!(rendered.len(), 1);
        assert_eq!(rendered[0].steps.len(), 1);
        let desc = rendered[0].steps[0].describe();
        assert!(desc.contains("dol-migrations"));
    }
}
