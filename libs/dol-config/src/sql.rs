/// SQL backend configuration with multi-instance (primary, replicas) support.
///
/// Exposes all connection pool, TLS, migration, and query logging options so that
/// callers never lose access to the underlying driver capabilities.
use serde::Deserialize;

use dol_sql::dialect::Dialect;

// ---------------------------------------------------------------------------
// Connection pool settings
// ---------------------------------------------------------------------------

/// Connection pool sizing and timeout knobs.
///
/// These map directly to the options exposed by pool managers such as sqlx,
/// deadpool, bb8, or r2d2.
#[derive(Debug, Clone, Deserialize)]
pub struct PoolConfig {
    /// Maximum number of connections in the pool.
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Minimum number of idle connections to keep in the pool.
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,

    /// Maximum time (seconds) to wait when acquiring a connection.
    #[serde(default = "default_acquire_timeout_secs")]
    pub acquire_timeout_secs: u64,

    /// Maximum idle time (seconds) before a connection is closed.
    #[serde(default = "default_idle_timeout_secs")]
    pub idle_timeout_secs: u64,

    /// Maximum lifetime (seconds) of a connection regardless of activity.
    #[serde(default = "default_max_lifetime_secs")]
    pub max_lifetime_secs: u64,
}

fn default_max_connections() -> u32 {
    10
}
fn default_min_connections() -> u32 {
    1
}
fn default_acquire_timeout_secs() -> u64 {
    30
}
fn default_idle_timeout_secs() -> u64 {
    600
}
fn default_max_lifetime_secs() -> u64 {
    1800
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
            min_connections: default_min_connections(),
            acquire_timeout_secs: default_acquire_timeout_secs(),
            idle_timeout_secs: default_idle_timeout_secs(),
            max_lifetime_secs: default_max_lifetime_secs(),
        }
    }
}

// ---------------------------------------------------------------------------
// TLS settings
// ---------------------------------------------------------------------------

/// TLS / SSL configuration for database connections.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TlsConfig {
    /// Whether to require TLS.
    #[serde(default)]
    pub enabled: bool,

    /// Path to a PEM-encoded CA certificate bundle for server verification.
    #[serde(default)]
    pub ca_cert_path: Option<String>,

    /// Path to a PEM-encoded client certificate (mutual TLS).
    #[serde(default)]
    pub client_cert_path: Option<String>,

    /// Path to a PEM-encoded client private key (mutual TLS).
    #[serde(default)]
    pub client_key_path: Option<String>,

    /// Accept any server certificate (development only — **not** for production).
    #[serde(default)]
    pub accept_invalid_certs: bool,
}

// ---------------------------------------------------------------------------
// Migration settings
// ---------------------------------------------------------------------------

/// Automatic migration configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct MigrationConfig {
    /// Run migrations automatically on startup.
    #[serde(default)]
    pub auto_run: bool,

    /// Directory containing migration scripts.
    #[serde(default = "default_migration_dir")]
    pub directory: String,
}

fn default_migration_dir() -> String {
    "migrations".into()
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            auto_run: false,
            directory: default_migration_dir(),
        }
    }
}

// ---------------------------------------------------------------------------
// Query logging settings
// ---------------------------------------------------------------------------

/// Query logging and slow-query diagnostics.
#[derive(Debug, Clone, Deserialize)]
pub struct QueryLoggingConfig {
    /// Log every SQL query.
    #[serde(default)]
    pub log_queries: bool,

    /// Log queries whose execution time exceeds `slow_query_threshold_ms`.
    #[serde(default)]
    pub log_slow_queries: bool,

    /// Threshold (milliseconds) above which a query is considered slow.
    #[serde(default = "default_slow_query_threshold_ms")]
    pub slow_query_threshold_ms: u64,
}

fn default_slow_query_threshold_ms() -> u64 {
    200
}

impl Default for QueryLoggingConfig {
    fn default() -> Self {
        Self {
            log_queries: false,
            log_slow_queries: false,
            slow_query_threshold_ms: default_slow_query_threshold_ms(),
        }
    }
}

// ---------------------------------------------------------------------------
// Single SQL instance (connection string + pool + TLS)
// ---------------------------------------------------------------------------

/// A single SQL database instance configuration.
///
/// Captures everything needed to open a connection to one database node:
/// connection URL, pool sizing, optional TLS, and an optional name for logging.
#[derive(Debug, Clone, Deserialize)]
pub struct SqlInstanceConfig {
    /// Human-readable name for this instance (used in logs / metrics).
    #[serde(default)]
    pub name: Option<String>,

    /// Database connection URL (e.g. `postgres://user:pass@host/db`).
    pub url: String,

    /// Connection pool settings.
    #[serde(default)]
    pub pool: PoolConfig,

    /// TLS / SSL settings.
    #[serde(default)]
    pub tls: TlsConfig,
}

// ---------------------------------------------------------------------------
// Top-level SQL config
// ---------------------------------------------------------------------------

/// Complete SQL backend configuration.
///
/// Supports enterprise topologies:
/// - **Single instance**: set `primary` only.
/// - **Read / write split**: set `primary` (writes) and `replicas` (reads).
/// - **Named instances**: use the `instances` map for arbitrary named
///   connections (analytics, reporting, etc.).
///
/// The `dialect` field can be a preset name (`"postgresql"`, `"mysql"`, …) or
/// an inline `Dialect` value loaded from config.
#[derive(Debug, Clone, Deserialize)]
pub struct SqlConfig {
    /// Which SQL dialect to use.
    ///
    /// Accepts either:
    /// - A preset name: `"postgresql"`, `"mysql"`, `"mariadb"`, `"sqlite"`,
    ///   `"mssql"`, `"oracle"`, `"cockroachdb"`.
    /// - An inline dialect table/object (same schema as `Dialect`).
    #[serde(default = "default_dialect_name")]
    pub dialect: DialectRef,

    /// Primary (read-write) database instance.
    pub primary: SqlInstanceConfig,

    /// Read-replica instances.
    ///
    /// When present, read queries may be routed here to offload the primary.
    #[serde(default)]
    pub replicas: Vec<SqlInstanceConfig>,

    /// Additional named instances for special workloads (analytics, reporting, etc.).
    #[serde(default)]
    pub instances: std::collections::HashMap<String, SqlInstanceConfig>,

    /// Migration settings.
    #[serde(default)]
    pub migrations: MigrationConfig,

    /// Query logging settings.
    #[serde(default)]
    pub logging: QueryLoggingConfig,
}

fn default_dialect_name() -> DialectRef {
    DialectRef::Preset("sqlite".into())
}

/// A reference to a `Dialect` — either a preset name or an inline definition.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DialectRef {
    /// A preset name such as `"postgresql"` or `"sqlite"`.
    Preset(String),
    /// A full inline `Dialect` definition.
    Inline(Box<Dialect>),
}

impl DialectRef {
    /// Resolve the reference to a concrete `Dialect`.
    ///
    /// Preset names are matched case-insensitively.
    pub fn resolve(&self) -> Dialect {
        match self {
            Self::Preset(name) => match name.to_ascii_lowercase().as_str() {
                "postgresql" | "postgres" | "pg" => Dialect::postgres(),
                "mysql" => Dialect::mysql(),
                "mariadb" => Dialect::mariadb(),
                "sqlite" => Dialect::sqlite(),
                "mssql" | "sqlserver" => Dialect::mssql(),
                "oracle" => Dialect::oracle(),
                "cockroachdb" | "crdb" => Dialect::cockroachdb(),
                _ => Dialect::sqlite(),
            },
            Self::Inline(d) => *d.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_config_defaults() {
        let pool = PoolConfig::default();
        assert_eq!(pool.max_connections, 10);
        assert_eq!(pool.min_connections, 1);
        assert_eq!(pool.acquire_timeout_secs, 30);
        assert_eq!(pool.idle_timeout_secs, 600);
        assert_eq!(pool.max_lifetime_secs, 1800);
    }

    #[test]
    fn tls_config_defaults() {
        let tls = TlsConfig::default();
        assert!(!tls.enabled);
        assert!(tls.ca_cert_path.is_none());
        assert!(!tls.accept_invalid_certs);
    }

    #[test]
    fn dialect_ref_resolve_presets() {
        let cases = [
            ("postgresql", "postgresql"),
            ("postgres", "postgresql"),
            ("pg", "postgresql"),
            ("mysql", "mysql"),
            ("mariadb", "mariadb"),
            ("sqlite", "sqlite"),
            ("mssql", "mssql"),
            ("sqlserver", "mssql"),
            ("oracle", "oracle"),
            ("cockroachdb", "cockroachdb"),
            ("crdb", "cockroachdb"),
        ];
        for (input, expected_name) in cases {
            let d = DialectRef::Preset(input.into()).resolve();
            assert_eq!(d.name, expected_name, "preset '{}' failed", input);
        }
    }

    #[test]
    fn sql_config_from_toml_single_instance() {
        let toml_str = r#"
            dialect = "postgresql"

            [primary]
            url = "postgres://user:pass@localhost/mydb"

            [primary.pool]
            max_connections = 20
            min_connections = 5

            [migrations]
            auto_run = true

            [logging]
            log_slow_queries = true
            slow_query_threshold_ms = 100
        "#;
        let cfg: SqlConfig = toml_crate::from_str(toml_str).unwrap();
        assert_eq!(cfg.primary.url, "postgres://user:pass@localhost/mydb");
        assert_eq!(cfg.primary.pool.max_connections, 20);
        assert!(cfg.replicas.is_empty());
        assert!(cfg.migrations.auto_run);
        assert!(cfg.logging.log_slow_queries);
    }

    #[test]
    fn sql_config_from_toml_with_replicas() {
        let toml_str = r#"
            dialect = "postgresql"

            [primary]
            url = "postgres://user:pass@primary:5432/mydb"

            [[replicas]]
            name = "replica-east"
            url = "postgres://user:pass@replica-east:5432/mydb"

            [replicas.pool]
            max_connections = 30

            [[replicas]]
            name = "replica-west"
            url = "postgres://user:pass@replica-west:5432/mydb"
        "#;
        let cfg: SqlConfig = toml_crate::from_str(toml_str).unwrap();
        assert_eq!(cfg.replicas.len(), 2);
        assert_eq!(cfg.replicas[0].name.as_deref(), Some("replica-east"));
        // First replica has explicit pool config
        assert_eq!(cfg.replicas[0].pool.max_connections, 30);
        assert_eq!(cfg.replicas[1].name.as_deref(), Some("replica-west"));
        // Second replica falls back to default pool config
        assert_eq!(cfg.replicas[1].pool.max_connections, 10); // default
    }

    #[test]
    fn sql_config_from_toml_named_instances() {
        let toml_str = r#"
            dialect = "postgresql"

            [primary]
            url = "postgres://user:pass@primary/mydb"

            [instances.analytics]
            url = "postgres://user:pass@analytics-host/analytics_db"

            [instances.analytics.pool]
            max_connections = 50
        "#;
        let cfg: SqlConfig = toml_crate::from_str(toml_str).unwrap();
        assert!(cfg.instances.contains_key("analytics"));
        assert_eq!(cfg.instances["analytics"].pool.max_connections, 50);
    }

    #[test]
    fn sql_config_with_tls() {
        let toml_str = r#"
            dialect = "postgresql"

            [primary]
            url = "postgres://user:pass@host/db?sslmode=verify-full"

            [primary.tls]
            enabled = true
            ca_cert_path = "/etc/ssl/certs/rds-combined-ca-bundle.pem"
        "#;
        let cfg: SqlConfig = toml_crate::from_str(toml_str).unwrap();
        assert!(cfg.primary.tls.enabled);
        assert_eq!(
            cfg.primary.tls.ca_cert_path.as_deref(),
            Some("/etc/ssl/certs/rds-combined-ca-bundle.pem")
        );
    }
}
