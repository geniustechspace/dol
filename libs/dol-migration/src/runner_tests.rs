use super::*;
use crate::test_helpers::{
    AddEmailIndex, CreateSessionsTable, CreateUsersTable, SetupKvNamespaces, SetupStorageBuckets,
};
use crate::{InMemoryRegistry, MigrationRegistry};

#[test]
fn runner_add_and_count() {
    let runner = MigrationRunner::new()
        .register(CreateUsersTable)
        .register(AddEmailIndex);
    assert_eq!(runner.count(), 2);
}

#[test]
fn runner_validate_unique_versions() {
    let runner = MigrationRunner::new()
        .register(CreateUsersTable)
        .register(AddEmailIndex);
    assert!(runner.validate().is_ok());
}

#[test]
fn runner_validate_duplicate_versions() {
    let runner = MigrationRunner::new()
        .register(CreateUsersTable)
        .register(CreateUsersTable);
    assert!(matches!(
        runner.validate(),
        Err(MigrationError::DuplicateVersion(_))
    ));
}

#[test]
fn runner_plan_forward_all() {
    let runner = MigrationRunner::new()
        .register(CreateUsersTable)
        .register(AddEmailIndex)
        .register(CreateSessionsTable);

    let registry = InMemoryRegistry::new();
    let plan = runner.plan_forward(&registry).unwrap();
    assert_eq!(plan.len(), 3);
    assert_eq!(plan.direction(), Some(MigrationDirection::Forward));
}

#[test]
fn runner_plan_forward_partial() {
    let runner = MigrationRunner::new()
        .register(CreateUsersTable)
        .register(AddEmailIndex)
        .register(CreateSessionsTable);

    let mut registry = InMemoryRegistry::new();
    registry
        .mark_applied("20240101_000001", "Create users", "abc")
        .unwrap();

    let plan = runner.plan_forward(&registry).unwrap();
    assert_eq!(plan.len(), 2);
    assert_eq!(plan.steps()[0].version, "20240102_000001");
}

#[test]
fn runner_plan_rollback() {
    let runner = MigrationRunner::new()
        .register(CreateUsersTable)
        .register(AddEmailIndex)
        .register(CreateSessionsTable);

    let mut registry = InMemoryRegistry::new();
    registry
        .mark_applied("20240101_000001", "Create users", "c1")
        .unwrap();
    registry
        .mark_applied("20240102_000001", "Add email index", "c2")
        .unwrap();
    registry
        .mark_applied("20240103_000001", "Create sessions", "c3")
        .unwrap();

    let plan = runner.plan_rollback(&registry, 1).unwrap();
    assert_eq!(plan.len(), 1);
    assert_eq!(plan.steps()[0].version, "20240103_000001");
    assert_eq!(plan.direction(), Some(MigrationDirection::Backward));
}

#[test]
fn runner_render_sql_plan() {
    let runner = MigrationRunner::new()
        .register(CreateUsersTable)
        .register(AddEmailIndex);

    let registry = InMemoryRegistry::new();
    let plan = runner.plan_forward(&registry).unwrap();
    let rendered = runner.render_plan(&plan, None);

    assert_eq!(rendered.len(), 2);
    assert_eq!(rendered[0].version, "20240101_000001");
    assert!(!rendered[0].steps.is_empty());

    // First migration should produce CREATE TABLE SQL
    let sql = rendered[0].steps[0].sql().unwrap();
    assert!(sql.contains("CREATE TABLE"));
    assert!(sql.contains("users"));
}

#[test]
fn runner_render_postgres() {
    let runner = MigrationRunner::new().register(CreateUsersTable);
    let registry = InMemoryRegistry::new();
    let plan = runner.plan_forward(&registry).unwrap();
    let pg = Dialect::postgres();
    let rendered = runner.render_plan(&plan, Some(&pg));

    let sql = rendered[0].steps[0].sql().unwrap();
    assert!(sql.contains("UUID"));
    assert!(sql.contains("CREATE TABLE"));
}

#[test]
fn runner_render_kv_steps() {
    let runner = MigrationRunner::new().register(SetupKvNamespaces);
    let registry = InMemoryRegistry::new();
    let plan = runner.plan_forward(&registry).unwrap();
    let rendered = runner.render_plan(&plan, None);

    assert_eq!(rendered.len(), 1);
    assert_eq!(rendered[0].steps.len(), 2);

    // Both steps should be KV operations
    for step in &rendered[0].steps {
        match step {
            RenderedStep::Kv { description, .. } => {
                assert!(!description.is_empty());
            }
            _ => panic!("expected KV step"),
        }
    }
}

#[test]
fn runner_render_storage_steps() {
    let runner = MigrationRunner::new().register(SetupStorageBuckets);
    let registry = InMemoryRegistry::new();
    let plan = runner.plan_forward(&registry).unwrap();
    let rendered = runner.render_plan(&plan, None);

    assert_eq!(rendered.len(), 1);
    assert_eq!(rendered[0].steps.len(), 2);

    // Both steps should be Storage operations
    for step in &rendered[0].steps {
        match step {
            RenderedStep::Storage { description, .. } => {
                assert!(!description.is_empty());
            }
            _ => panic!("expected Storage step"),
        }
    }
}

#[test]
fn runner_status() {
    let runner = MigrationRunner::new()
        .register(CreateUsersTable)
        .register(AddEmailIndex)
        .register(CreateSessionsTable);

    let mut registry = InMemoryRegistry::new();
    registry
        .mark_applied("20240101_000001", "Create users", "c1")
        .unwrap();

    let statuses = runner.status(&registry).unwrap();
    assert_eq!(statuses.len(), 3);
    assert_eq!(statuses[0].state, MigrationState::Applied);
    assert_eq!(statuses[1].state, MigrationState::Pending);
    assert_eq!(statuses[2].state, MigrationState::Pending);
}

#[test]
fn runner_checksum_validation() {
    let runner = MigrationRunner::new().register(CreateUsersTable);

    let mut registry = InMemoryRegistry::new();
    // Apply with the correct checksum
    let checksum = plan::compute_checksum(&CreateUsersTable.up());
    registry
        .mark_applied("20240101_000001", "Create users", &checksum)
        .unwrap();

    // Should pass
    assert!(runner.validate_checksums(&registry).is_ok());
}

#[test]
fn runner_checksum_mismatch() {
    let runner = MigrationRunner::new().register(CreateUsersTable);

    let mut registry = InMemoryRegistry::new();
    // Apply with a wrong checksum
    registry
        .mark_applied("20240101_000001", "Create users", "wrong_checksum")
        .unwrap();

    let result = runner.validate_checksums(&registry);
    assert!(matches!(
        result,
        Err(MigrationError::ChecksumMismatch { .. })
    ));
}

#[test]
fn runner_empty_checksum_skipped() {
    let runner = MigrationRunner::new().register(CreateUsersTable);

    let mut registry = InMemoryRegistry::new();
    // Empty checksum should be skipped (legacy data)
    registry
        .mark_applied("20240101_000001", "Create users", "")
        .unwrap();

    assert!(runner.validate_checksums(&registry).is_ok());
}

#[test]
fn rendered_step_describe() {
    let step = RenderedStep::Sql {
        sql: "CREATE TABLE users (id UUID PRIMARY KEY)".to_string(),
        param_count: 0,
    };
    assert!(step.describe().contains("CREATE TABLE"));
    assert_eq!(step.param_count(), Some(0));

    let step = RenderedStep::Kv {
        description: "CREATE NAMESPACE 'users:'".to_string(),
        operation: KvMigrationOp::CreateNamespace {
            prefix: "users:".to_string(),
        },
    };
    assert!(step.describe().contains("NAMESPACE"));
    assert!(step.sql().is_none());
}

#[test]
fn migration_plan_empty_is_up_to_date() {
    let runner = MigrationRunner::new().register(CreateUsersTable);

    let mut registry = InMemoryRegistry::new();
    registry
        .mark_applied("20240101_000001", "Create users", "c1")
        .unwrap();

    let plan = runner.plan_forward(&registry).unwrap();
    assert!(plan.is_empty());
    assert_eq!(plan.direction(), None);
}

#[test]
fn render_plan_filtered_sql_only() {
    let runner = MigrationRunner::new()
        .register(CreateUsersTable)
        .register(SetupKvNamespaces)
        .register(SetupStorageBuckets);

    let registry = InMemoryRegistry::new();
    let plan = runner.plan_forward(&registry).unwrap();

    // SQL only — KV and Storage steps should be excluded
    let rendered = runner.render_plan_filtered(&plan, None, true, false, false);
    assert_eq!(rendered.len(), 1); // Only CreateUsersTable has SQL steps
    assert!(rendered[0].steps[0].sql().is_some());
}

#[test]
fn render_plan_filtered_kv_only() {
    let runner = MigrationRunner::new()
        .register(CreateUsersTable)
        .register(SetupKvNamespaces);

    let registry = InMemoryRegistry::new();
    let plan = runner.plan_forward(&registry).unwrap();

    // KV only — SQL steps should be excluded
    let rendered = runner.render_plan_filtered(&plan, None, false, true, false);
    assert_eq!(rendered.len(), 1); // Only SetupKvNamespaces has KV steps
    for step in &rendered[0].steps {
        assert!(matches!(step, RenderedStep::Kv { .. }));
    }
}

#[test]
fn render_plan_filtered_all_enabled() {
    let runner = MigrationRunner::new()
        .register(CreateUsersTable)
        .register(SetupKvNamespaces)
        .register(SetupStorageBuckets);

    let registry = InMemoryRegistry::new();
    let plan = runner.plan_forward(&registry).unwrap();

    // All backends enabled — should include all migrations
    let rendered = runner.render_plan_filtered(&plan, None, true, true, true);
    assert_eq!(rendered.len(), 3);
}
