use super::*;
use dol_entity::DefineEntityBuilder;
use dol_ir::entity_ref::EntityRef;
use dol_ir::definition::{DefineIndex, FieldDef};
use dol_entity::DataType;

/// Test migration: Create users table.
pub struct CreateUsersTable;

impl Migration for CreateUsersTable {
    fn version(&self) -> &str {
        "20240101_000001"
    }
    fn description(&self) -> &str {
        "Create users table"
    }
    fn up(&self) -> Vec<MigrationStep> {
        vec![MigrationStep::define_entity(
            DefineEntityBuilder::new("users")
                .field(FieldDef::new("id", DataType::Uuid).primary_key())
                .field(FieldDef::new("email", DataType::Text).unique())
                .field(FieldDef::new("status", DataType::Text).default("'active'"))
                .field(FieldDef::new("created_at", DataType::TimestampTz { precision: 6 }).default("NOW()"))
                .if_not_exists()
                .build(),
        )]
    }
    fn down(&self) -> Vec<MigrationStep> {
        vec![MigrationStep::drop_entity("users")]
    }
}

/// Test migration: Add email index.
pub struct AddEmailIndex;

impl Migration for AddEmailIndex {
    fn version(&self) -> &str {
        "20240102_000001"
    }
    fn description(&self) -> &str {
        "Add unique index on users.email"
    }
    fn up(&self) -> Vec<MigrationStep> {
        vec![MigrationStep::define_index(DefineIndex {
            name: "idx_users_email".to_string(),
            target: EntityRef {
                name: "users".to_string(),
                namespace: None,
                alias: None,
            },
            columns: vec!["email".to_string()],
            unique: true,
            if_not_exists: true,
            concurrently: false,
            method: None,
            where_clause: None,
        })]
    }
    fn down(&self) -> Vec<MigrationStep> {
        vec![MigrationStep::drop_index("idx_users_email")]
    }
}

/// Test migration: Create sessions table.
pub struct CreateSessionsTable;

impl Migration for CreateSessionsTable {
    fn version(&self) -> &str {
        "20240103_000001"
    }
    fn description(&self) -> &str {
        "Create sessions table"
    }
    fn up(&self) -> Vec<MigrationStep> {
        vec![MigrationStep::define_entity(
            DefineEntityBuilder::new("sessions")
                .field(FieldDef::new("id", DataType::Uuid).primary_key())
                .field(FieldDef::new("user_id", DataType::Uuid))
                .field(FieldDef::new("token", DataType::Text))
                .field(FieldDef::new("expires_at", DataType::TimestampTz { precision: 6 }))
                .if_not_exists()
                .build(),
        )]
    }
    fn down(&self) -> Vec<MigrationStep> {
        vec![MigrationStep::drop_entity("sessions")]
    }
}

/// Test migration: Set up KV namespaces.
pub struct SetupKvNamespaces;

impl Migration for SetupKvNamespaces {
    fn version(&self) -> &str {
        "20240104_000001"
    }
    fn description(&self) -> &str {
        "Set up KV namespaces for sessions and cache"
    }
    fn up(&self) -> Vec<MigrationStep> {
        vec![
            MigrationStep::create_kv_namespace("sessions:"),
            MigrationStep::create_kv_namespace("cache:"),
        ]
    }
    fn down(&self) -> Vec<MigrationStep> {
        vec![
            MigrationStep::drop_kv_namespace("cache:"),
            MigrationStep::drop_kv_namespace("sessions:"),
        ]
    }
}

/// Test migration: Set up storage buckets.
pub struct SetupStorageBuckets;

impl Migration for SetupStorageBuckets {
    fn version(&self) -> &str {
        "20240105_000001"
    }
    fn description(&self) -> &str {
        "Set up object storage buckets"
    }
    fn up(&self) -> Vec<MigrationStep> {
        vec![
            MigrationStep::create_bucket_in_region("app-uploads", "us-east-1"),
            MigrationStep::create_bucket("app-backups"),
        ]
    }
    fn down(&self) -> Vec<MigrationStep> {
        vec![
            MigrationStep::delete_bucket("app-backups"),
            MigrationStep::delete_bucket("app-uploads"),
        ]
    }
}
