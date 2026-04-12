//! Unified configuration for the DOL crate.
//!
//! `DolConfig` is the single entry-point for configuring all DOL backends
//! (SQL, key-value, object storage) and their dialects from one file.
//!
//! # Enterprise multi-instance support
//!
//! Each backend supports:
//! - A **primary** instance (required).
//! - Optional **replicas** for read scaling / geo-redundancy.
//! - Optional **named instances** for isolated workloads (analytics,
//!   sessions, backups, etc.).
//!
//! # Config file formats
//!
//! Load from TOML, YAML, or JSON — format is auto-detected from the file
//! extension (`.toml`, `.yaml`/`.yml`, `.json`).
//!
//! # Example (TOML)
//!
//! ```toml
//! [sql]
//! dialect = "postgresql"
//!
//! [sql.primary]
//! url = "postgres://user:pass@primary:5432/mydb"
//!
//! [sql.primary.pool]
//! max_connections = 20
//!
//! [[sql.replicas]]
//! name = "replica-east"
//! url = "postgres://user:pass@replica-east:5432/mydb"
//!
//! [kv]
//! provider = "Redis"
//!
//! [kv.primary]
//! url = "redis://localhost:6379"
//! key_prefix = "app:"
//!
//! [storage]
//! provider = "S3"
//!
//! [storage.primary]
//! bucket = "my-app-assets"
//! region = "us-east-1"
//! ```

pub mod kv;
pub mod migration;
pub mod sql;
pub mod storage;

pub use kv::{KvConfig, KvInstanceConfig, KvProvider, RedisConfig, RedisSentinelConfig};
pub use migration::{BackendFilter, LockStrategy, UnifiedMigrationConfig};
pub use sql::{
    DialectRef, MigrationConfig, PoolConfig, QueryLoggingConfig, SqlConfig, SqlInstanceConfig,
    TlsConfig,
};
pub use storage::{
    AzureBlobConfig, GcsConfig, LocalStorageConfig, S3Config, StorageConfig, StorageInstanceConfig,
    StorageProvider,
};

use serde::Deserialize;
use std::fmt;
use std::fs;

// ---------------------------------------------------------------------------
// Unified DOL config
// ---------------------------------------------------------------------------

/// Complete DOL configuration — all backends in a single struct.
///
/// Every section is optional so you can configure only the backends your
/// application actually uses.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DolConfig {
    /// SQL database backend configuration.
    #[serde(default)]
    pub sql: Option<SqlConfig>,

    /// Key-value store backend configuration.
    #[serde(default)]
    pub kv: Option<KvConfig>,

    /// Object / blob storage backend configuration.
    #[serde(default)]
    pub storage: Option<StorageConfig>,

    /// Unified migration settings (applies to all backends).
    #[serde(default)]
    pub migrations: UnifiedMigrationConfig,
}

// ---------------------------------------------------------------------------
// Config loading errors
// ---------------------------------------------------------------------------

/// Errors that can occur when loading a `DolConfig` from a file or string.
#[derive(Debug)]
pub enum ConfigError {
    /// File I/O error.
    Io(std::io::Error),
    /// JSON deserialization error.
    Json(serde_json::Error),
    /// TOML deserialization error.
    Toml(toml_crate::de::Error),
    /// YAML deserialization error.
    Yaml(serde_yaml::Error),
    /// Unrecognized file extension.
    UnsupportedFormat(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::Json(e) => write!(f, "JSON parse error: {}", e),
            Self::Toml(e) => write!(f, "TOML parse error: {}", e),
            Self::Yaml(e) => write!(f, "YAML parse error: {}", e),
            Self::UnsupportedFormat(ext) => {
                write!(f, "unsupported config format: .{}", ext)
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::Toml(e) => Some(e),
            Self::Yaml(e) => Some(e),
            Self::UnsupportedFormat(_) => None,
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

impl From<toml_crate::de::Error> for ConfigError {
    fn from(e: toml_crate::de::Error) -> Self {
        Self::Toml(e)
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(e: serde_yaml::Error) -> Self {
        Self::Yaml(e)
    }
}

// ---------------------------------------------------------------------------
// Config loading methods
// ---------------------------------------------------------------------------

impl DolConfig {
    /// Deserialize from a JSON string.
    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Deserialize from a TOML string.
    pub fn from_toml_str(toml: &str) -> Result<Self, toml_crate::de::Error> {
        toml_crate::from_str(toml)
    }

    /// Deserialize from a YAML string.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Load from a file, auto-detecting format by extension.
    ///
    /// Supported extensions: `.json`, `.toml`, `.yaml`, `.yml`.
    pub fn from_file(path: &str) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path)?;
        let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        match ext.as_str() {
            "json" => Ok(Self::from_json_str(&contents)?),
            "toml" => Ok(Self::from_toml_str(&contents)?),
            "yaml" | "yml" => Ok(Self::from_yaml_str(&contents)?),
            other => Err(ConfigError::UnsupportedFormat(other.to_string())),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
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

        let cfg: DolConfig = serde_yaml::from_str(yaml_str).unwrap();
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
                    "type_map": { "mappings": {
                        "Uuid": "UUID", "Text": "TEXT", "Integer": "INTEGER",
                        "Boolean": "BOOLEAN", "Timestamptz": "TIMESTAMPTZ",
                        "Jsonb": "JSONB", "TextArray": "TEXT[]", "SmallInt": "SMALLINT",
                        "BigInt": "BIGINT", "Real": "REAL", "DoublePrecision": "DOUBLE PRECISION",
                        "Numeric": "NUMERIC", "Bytea": "BYTEA", "Date": "DATE",
                        "Time": "TIME", "Interval": "INTERVAL", "Serial": "SERIAL",
                        "BigSerial": "BIGSERIAL", "Inet": "INET"
                    }},
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
}
