use super::*;
use crate::async_registry::{AsyncMigrationRegistry, InMemoryAsyncRegistry};
use crate::test_helpers::*;

#[tokio::test]
async fn async_plan_forward() {
    let runner = MigrationRunner::new()
        .register(CreateUsersTable)
        .register(AddEmailIndex);

    let registry = InMemoryAsyncRegistry::new();
    let plan = runner.plan_forward_async(&registry).await.unwrap();
    assert_eq!(plan.steps().len(), 2);
    assert_eq!(plan.steps()[0].version, "20240101_000001");
    assert_eq!(plan.steps()[1].version, "20240102_000001");
}

#[tokio::test]
async fn async_plan_with_applied() {
    let runner = MigrationRunner::new()
        .register(CreateUsersTable)
        .register(AddEmailIndex)
        .register(CreateSessionsTable);

    let mut registry = InMemoryAsyncRegistry::new();
    registry
        .mark_applied("20240101_000001", "Create users table", "checksum")
        .await
        .unwrap();

    let plan = runner.plan_forward_async(&registry).await.unwrap();
    assert_eq!(plan.steps().len(), 2);
    assert_eq!(plan.steps()[0].version, "20240102_000001");
    assert_eq!(plan.steps()[1].version, "20240103_000001");
}

#[tokio::test]
async fn async_plan_rollback() {
    let runner = MigrationRunner::new()
        .register(CreateUsersTable)
        .register(AddEmailIndex);

    let mut registry = InMemoryAsyncRegistry::new();
    registry
        .mark_applied("20240101_000001", "Create users table", "c1")
        .await
        .unwrap();
    registry
        .mark_applied("20240102_000001", "Add email index", "c2")
        .await
        .unwrap();

    let plan = runner.plan_rollback_async(&registry, 1).await.unwrap();
    assert_eq!(plan.steps().len(), 1);
    assert_eq!(plan.steps()[0].version, "20240102_000001");
}

#[tokio::test]
async fn async_status() {
    let runner = MigrationRunner::new()
        .register(CreateUsersTable)
        .register(AddEmailIndex)
        .register(CreateSessionsTable);

    let mut registry = InMemoryAsyncRegistry::new();
    registry
        .mark_applied("20240101_000001", "Create users table", "c1")
        .await
        .unwrap();

    let status = runner.status_async(&registry).await.unwrap();
    assert_eq!(status.len(), 3);
    assert_eq!(status[0].state, runner::MigrationState::Applied);
    assert_eq!(status[1].state, runner::MigrationState::Pending);
    assert_eq!(status[2].state, runner::MigrationState::Pending);
}

#[tokio::test]
async fn async_registry_basic() {
    let mut reg = InMemoryAsyncRegistry::new();
    assert!(reg.applied().await.unwrap().is_empty());

    reg.mark_applied("V001", "First migration", "checksum1")
        .await
        .unwrap();
    reg.mark_applied("V002", "Second migration", "checksum2")
        .await
        .unwrap();

    let applied = reg.applied().await.unwrap();
    assert_eq!(applied.len(), 2);
    assert_eq!(applied[0].version, "V001");
    assert_eq!(applied[1].version, "V002");
}

#[tokio::test]
async fn async_registry_revert() {
    let mut reg = InMemoryAsyncRegistry::new();
    reg.mark_applied("V001", "First", "c1").await.unwrap();
    reg.mark_applied("V002", "Second", "c2").await.unwrap();

    reg.mark_reverted("V001").await.unwrap();
    let applied = reg.applied().await.unwrap();
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].version, "V002");
}

#[tokio::test]
async fn async_registry_duplicate() {
    let mut reg = InMemoryAsyncRegistry::new();
    reg.mark_applied("V001", "First", "c1").await.unwrap();
    let result = reg.mark_applied("V001", "Duplicate", "c2").await;
    assert!(matches!(result, Err(MigrationError::DuplicateVersion(_))));
}

#[tokio::test]
async fn async_sync_registry_as_async() {
    // Verify that InMemoryRegistry (sync) works through the async trait via blanket impl
    let runner = MigrationRunner::new()
        .register(CreateUsersTable)
        .register(AddEmailIndex);

    let registry = InMemoryRegistry::new();
    let plan = runner.plan_forward_async(&registry).await.unwrap();
    assert_eq!(plan.steps().len(), 2);
}
