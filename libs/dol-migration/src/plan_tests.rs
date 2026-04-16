use super::*;
use crate::test_helpers::{AddEmailIndex, CreateSessionsTable, CreateUsersTable};

#[test]
fn plan_forward_all_pending() {
    let migrations: Vec<Box<dyn Migration>> = vec![
        Box::new(CreateUsersTable),
        Box::new(AddEmailIndex),
        Box::new(CreateSessionsTable),
    ];
    let applied: Vec<AppliedMigration> = vec![];
    let plan = plan_forward(&migrations, &applied).unwrap();
    assert_eq!(plan.len(), 3);
    assert_eq!(plan.steps()[0].version, "20240101_000001");
    assert_eq!(plan.steps()[1].version, "20240102_000001");
    assert_eq!(plan.steps()[2].version, "20240103_000001");
}

#[test]
fn plan_forward_partial() {
    let migrations: Vec<Box<dyn Migration>> = vec![
        Box::new(CreateUsersTable),
        Box::new(AddEmailIndex),
        Box::new(CreateSessionsTable),
    ];
    let applied = vec![AppliedMigration {
        version: "20240101_000001".into(),
        description: "Create users table".into(),
        checksum: "abc".into(),
        applied_at: None,
        execution_time_ms: None,
    }];
    let plan = plan_forward(&migrations, &applied).unwrap();
    assert_eq!(plan.len(), 2);
    assert_eq!(plan.steps()[0].version, "20240102_000001");
    assert_eq!(plan.steps()[1].version, "20240103_000001");
}

#[test]
fn plan_forward_none_pending() {
    let migrations: Vec<Box<dyn Migration>> = vec![Box::new(CreateUsersTable)];
    let applied = vec![AppliedMigration {
        version: "20240101_000001".into(),
        description: "".into(),
        checksum: "".into(),
        applied_at: None,
        execution_time_ms: None,
    }];
    let plan = plan_forward(&migrations, &applied).unwrap();
    assert!(plan.is_empty());
}

#[test]
fn plan_to_specific_version() {
    let migrations: Vec<Box<dyn Migration>> = vec![
        Box::new(CreateUsersTable),
        Box::new(AddEmailIndex),
        Box::new(CreateSessionsTable),
    ];
    let applied: Vec<AppliedMigration> = vec![];
    let plan = plan_to_target(
        &migrations,
        &applied,
        &MigrationTarget::Version("20240102_000001".into()),
    )
    .unwrap();
    assert_eq!(plan.len(), 2);
    assert_eq!(plan.steps()[0].version, "20240101_000001");
    assert_eq!(plan.steps()[1].version, "20240102_000001");
}

#[test]
fn plan_rollback_n() {
    let migrations: Vec<Box<dyn Migration>> = vec![
        Box::new(CreateUsersTable),
        Box::new(AddEmailIndex),
        Box::new(CreateSessionsTable),
    ];
    let applied = vec![
        AppliedMigration {
            version: "20240101_000001".into(),
            description: "".into(),
            checksum: "".into(),
            applied_at: None,
            execution_time_ms: None,
        },
        AppliedMigration {
            version: "20240102_000001".into(),
            description: "".into(),
            checksum: "".into(),
            applied_at: None,
            execution_time_ms: None,
        },
        AppliedMigration {
            version: "20240103_000001".into(),
            description: "".into(),
            checksum: "".into(),
            applied_at: None,
            execution_time_ms: None,
        },
    ];
    let plan = plan_to_target(&migrations, &applied, &MigrationTarget::Rollback(2)).unwrap();
    assert_eq!(plan.len(), 2);
    assert_eq!(plan.direction(), Some(MigrationDirection::Backward));
    assert_eq!(plan.steps()[0].version, "20240103_000001");
    assert_eq!(plan.steps()[1].version, "20240102_000001");
}

#[test]
fn plan_reset() {
    let migrations: Vec<Box<dyn Migration>> =
        vec![Box::new(CreateUsersTable), Box::new(AddEmailIndex)];
    let applied = vec![
        AppliedMigration {
            version: "20240101_000001".into(),
            description: "".into(),
            checksum: "".into(),
            applied_at: None,
            execution_time_ms: None,
        },
        AppliedMigration {
            version: "20240102_000001".into(),
            description: "".into(),
            checksum: "".into(),
            applied_at: None,
            execution_time_ms: None,
        },
    ];
    let plan = plan_to_target(&migrations, &applied, &MigrationTarget::Reset).unwrap();
    assert_eq!(plan.len(), 2);
    // Reversed order
    assert_eq!(plan.steps()[0].version, "20240102_000001");
    assert_eq!(plan.steps()[1].version, "20240101_000001");
}

#[test]
fn plan_target_not_found() {
    let migrations: Vec<Box<dyn Migration>> = vec![Box::new(CreateUsersTable)];
    let applied: Vec<AppliedMigration> = vec![];
    let result = plan_to_target(
        &migrations,
        &applied,
        &MigrationTarget::Version("nonexistent".into()),
    );
    assert!(matches!(result, Err(MigrationError::TargetNotFound(_))));
}

#[test]
fn checksum_deterministic() {
    let steps = CreateUsersTable.up();
    let c1 = compute_checksum(&steps);
    let c2 = compute_checksum(&steps);
    assert_eq!(c1, c2);
    assert_eq!(c1.len(), 16); // 16 hex chars
}

#[test]
fn checksum_different_for_different_steps() {
    let c1 = compute_checksum(&CreateUsersTable.up());
    let c2 = compute_checksum(&AddEmailIndex.up());
    assert_ne!(c1, c2);
}
