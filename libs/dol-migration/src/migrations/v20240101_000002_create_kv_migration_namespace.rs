//! Migration: Create the `dol:migrations:` KV namespace.
//!
//! This migration sets up the key-value namespace that DOL uses to track
//! migration state in KV backends (Redis, etcd, DynamoDB, etc.).

use crate::{Migration, MigrationStep};

/// Creates the `dol:migrations:` key namespace for tracking migration state.
///
/// **Forward (up)**:
/// - Creates the `dol:migrations:` namespace prefix
///
/// **Backward (down)**:
/// - Drops the `dol:migrations:` namespace (removes all keys under the prefix)
pub struct CreateKvMigrationNamespace;

impl Migration for CreateKvMigrationNamespace {
    fn version(&self) -> &str {
        "20240101_000002"
    }

    fn description(&self) -> &str {
        "Create dol:migrations: KV namespace"
    }

    fn up(&self) -> Vec<MigrationStep> {
        vec![MigrationStep::create_kv_namespace("dol:migrations:")]
    }

    fn down(&self) -> Vec<MigrationStep> {
        vec![MigrationStep::drop_kv_namespace("dol:migrations:")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_version_and_description() {
        let m = CreateKvMigrationNamespace;
        assert_eq!(m.version(), "20240101_000002");
        assert!(!m.description().is_empty());
    }

    #[test]
    fn up_creates_namespace() {
        let steps = CreateKvMigrationNamespace.up();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].kind(), "kv");
    }

    #[test]
    fn down_drops_namespace() {
        let steps = CreateKvMigrationNamespace.down();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].kind(), "kv");
    }

    #[test]
    fn renders_kv_operations() {
        let runner =
            crate::MigrationRunner::new().register(CreateKvMigrationNamespace);
        let registry = crate::InMemoryRegistry::new();
        let plan = runner.plan_forward(&registry).unwrap();
        let rendered = runner.render_plan(&plan, None);

        assert_eq!(rendered.len(), 1);
        assert_eq!(rendered[0].steps.len(), 1);
        // KV steps render as descriptions
        let desc = rendered[0].steps[0].describe();
        assert!(desc.contains("dol:migrations:"));
    }
}
