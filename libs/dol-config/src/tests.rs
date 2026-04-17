use super::*;

#[test]
fn empty_config_deserializes() {
    let cfg: DolConfig = toml_crate::from_str("").unwrap();
    assert!(cfg.sql.is_none());
    assert!(cfg.kv.is_none());
    assert!(cfg.storage.is_none());
}

#[test]
fn sql_only_config() {
    let toml_str = r#"
            [sql]
            dialect = "postgresql"

            [sql.primary]
            url = "postgres://user:pass@localhost/mydb"
        "#;
    let cfg: DolConfig = toml_crate::from_str(toml_str).unwrap();
    assert!(cfg.sql.is_some());
    assert!(cfg.kv.is_none());
    assert!(cfg.storage.is_none());
    let sql = cfg.sql.unwrap();
    assert_eq!(sql.primary.url, "postgres://user:pass@localhost/mydb");
}

#[test]
fn full_enterprise_config_toml() {
    let toml_str = r#"
            # --- SQL ---
            [sql]
            dialect = "postgresql"

            [sql.primary]
            url = "postgres://user:pass@primary:5432/mydb"

            [sql.primary.pool]
            max_connections = 20
            min_connections = 5

            [sql.primary.tls]
            enabled = true
            ca_cert_path = "/etc/ssl/rds-ca.pem"

            [[sql.replicas]]
            name = "replica-east"
            url = "postgres://user:pass@replica-east:5432/mydb"

            [[sql.replicas]]
            name = "replica-west"
            url = "postgres://user:pass@replica-west:5432/mydb"

            [sql.instances.analytics]
            url = "postgres://analytics:pass@analytics-host/analytics_db"

            [sql.instances.analytics.pool]
            max_connections = 50

            [sql.migrations]
            auto_run = true
            directory = "db/migrations"

            [sql.logging]
            log_slow_queries = true
            slow_query_threshold_ms = 100

            # --- KV ---
            [kv]
            provider = "Redis"
            key_prefix = "idp:"
            default_ttl_secs = 3600

            [kv.primary]
            url = "redis://redis-primary:6379"

            [[kv.replicas]]
            name = "redis-replica-1"
            url = "redis://redis-replica-1:6379"

            [kv.instances.sessions]
            url = "redis://sessions-host:6379"

            [kv.instances.rate_limits]
            url = "redis://rate-limits-host:6379"

            # --- Storage ---
            [storage]
            provider = "S3"

            [storage.primary]
            bucket = "idp-assets"
            region = "us-east-1"

            [[storage.replicas]]
            name = "eu-replica"
            bucket = "idp-assets-eu"
            region = "eu-west-1"

            [storage.instances.backups]
            bucket = "idp-backups"
            region = "us-west-2"

            [storage.s3]
            storage_class = "INTELLIGENT_TIERING"
            server_side_encryption = "aws:kms"
            kms_key_id = "arn:aws:kms:us-east-1:123456789:key/abc-123"
        "#;

    let cfg: DolConfig = toml_crate::from_str(toml_str).unwrap();

    // SQL assertions
    let sql = cfg.sql.unwrap();
    assert_eq!(sql.primary.pool.max_connections, 20);
    assert!(sql.primary.tls.enabled);
    assert_eq!(sql.replicas.len(), 2);
    assert!(sql.instances.contains_key("analytics"));
    assert!(sql.migrations.auto_run);

    // KV assertions
    let kv = cfg.kv.unwrap();
    assert_eq!(kv.provider, KvProvider::Redis);
    assert_eq!(kv.key_prefix.as_deref(), Some("idp:"));
    assert_eq!(kv.replicas.len(), 1);
    assert!(kv.instances.contains_key("sessions"));
    assert!(kv.instances.contains_key("rate_limits"));

    // Storage assertions
    let storage = cfg.storage.unwrap();
    assert_eq!(storage.provider, StorageProvider::S3);
    assert_eq!(storage.primary.bucket, "idp-assets");
    assert_eq!(storage.replicas.len(), 1);
    assert!(storage.instances.contains_key("backups"));
    let s3 = storage.s3.unwrap();
    assert_eq!(s3.storage_class.as_deref(), Some("INTELLIGENT_TIERING"));
}

#[test]
fn full_enterprise_config_yaml() {
    let yaml_str = r#"
sql:
  dialect: postgresql
  primary:
    url: "postgres://user:pass@primary:5432/mydb"
    pool:
      max_connections: 20
  replicas:
    - name: replica-1
      url: "postgres://user:pass@replica-1:5432/mydb"
kv:
  provider: Redis
  primary:
    url: "redis://localhost:6379"
  key_prefix: "app:"
storage:
  provider: S3
  primary:
    bucket: my-bucket
    region: us-east-1
"#;

    let cfg: DolConfig = serde_norway::from_str(yaml_str).unwrap();
    assert!(cfg.sql.is_some());
    assert!(cfg.kv.is_some());
    assert!(cfg.storage.is_some());
}

#[test]
fn full_enterprise_config_json() {
    let json_str = r#"{
            "sql": {
                "dialect": "postgresql",
                "primary": { "url": "postgres://user:pass@host/db" }
            },
            "kv": {
                "provider": "Redis",
                "primary": { "url": "redis://localhost:6379" }
            },
            "storage": {
                "provider": "S3",
                "primary": { "bucket": "my-bucket" }
            }
        }"#;

    let cfg: DolConfig = serde_json::from_str(json_str).unwrap();
    assert!(cfg.sql.is_some());
    assert!(cfg.kv.is_some());
    assert!(cfg.storage.is_some());
}

#[test]
fn dialect_ref_inline_via_json() {
    // JSON handles untagged enum deserialization cleanly for inline dialects
    let json_str = r#"{
            "sql": {
                "dialect": {
                    "name": "custom_pg",
                    "param_style": { "style": "Numbered", "prefix": "$" },
                    "quote_style": "DoubleQuote",
                    "pagination": "LimitOffset",
                    "upsert_style": "OnConflict",
                    "returning_style": "Returning",
                    "bool_true": "TRUE",
                    "bool_false": "FALSE",
                    "concat_style": "PipeOperator",
                    "type_dialect": "Postgres",
                    "locking": { "for_update": true, "for_share": true, "skip_locked": true, "nowait": true, "use_table_hint": false },
                    "ddl": {
                        "create_if_not_exists": true, "drop_if_exists": true,
                        "index_concurrently": true, "enum_style": "CreateType",
                        "auto_increment_style": "SerialType",
                        "alter_add_column": true, "alter_drop_column": true,
                        "alter_rename_column": true, "alter_modify_column": true,
                        "alter_rename_table": true, "transactional_ddl": true, "drop_cascade": true
                    },
                    "features": {
                        "distinct_on": true, "ilike": true, "array_any": true,
                        "nulls_ordering": true, "anonymous_blocks": true, "schemas": true,
                        "cte": true, "window_functions": true, "lateral_join": true, "on_conflict": true
                    }
                },
                "primary": { "url": "postgres://localhost/mydb" }
            }
        }"#;
    let cfg: DolConfig = serde_json::from_str(json_str).unwrap();
    let sql = cfg.sql.unwrap();
    let dialect = sql.dialect.resolve();
    assert_eq!(dialect.name, "custom_pg");
    assert_eq!(dialect.bool_true, "TRUE");
    assert!(dialect.features.distinct_on);
}

#[test]
fn unsupported_format_error() {
    let result = DolConfig::from_file("/nonexistent.xyz");
    assert!(result.is_err());
}

#[test]
fn unified_migration_config_in_dol_config() {
    let toml_str = r#"
            [migrations]
            auto_run = true
            table_name = "schema_history"
            lock_strategy = "advisory_lock"
            validate_checksums = true
            dry_run = false

            [migrations.backends]
            sql = true
            kv = true
            storage = false
        "#;
    let cfg: DolConfig = toml_crate::from_str(toml_str).unwrap();
    assert!(cfg.migrations.auto_run);
    assert_eq!(cfg.migrations.table_name, "schema_history");
    assert!(cfg.migrations.backends.sql);
    assert!(cfg.migrations.backends.kv);
    assert!(!cfg.migrations.backends.storage);
    assert!(matches!(
        cfg.migrations.lock_strategy,
        LockStrategy::AdvisoryLock
    ));
}

#[test]
fn default_migration_config_when_omitted() {
    let cfg: DolConfig = toml_crate::from_str("").unwrap();
    assert!(!cfg.migrations.auto_run);
    assert_eq!(cfg.migrations.table_name, "_dol_migrations");
    assert!(cfg.migrations.backends.sql);
    assert!(cfg.migrations.backends.kv);
    assert!(cfg.migrations.backends.storage);
}

#[test]
fn full_config_with_migrations_and_backends() {
    let toml_str = r#"
            [sql]
            dialect = "postgresql"
            [sql.primary]
            url = "postgres://localhost/db"

            [kv]
            provider = "Redis"
            [kv.primary]
            url = "redis://localhost"

            [migrations]
            auto_run = true
            lock_strategy = "lock_table"

            [migrations.backends]
            sql = true
            kv = true
            storage = false
        "#;
    let cfg: DolConfig = toml_crate::from_str(toml_str).unwrap();
    assert!(cfg.sql.is_some());
    assert!(cfg.kv.is_some());
    assert!(cfg.migrations.auto_run);
    assert!(matches!(
        cfg.migrations.lock_strategy,
        LockStrategy::LockTable
    ));
    assert!(!cfg.migrations.backends.storage);
}
