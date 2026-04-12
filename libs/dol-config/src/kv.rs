/// Key-value backend configuration with multi-instance (primary, replicas) support.
///
/// Covers Redis, etcd, DynamoDB, Memcached and other KV providers.
/// All provider-specific knobs are exposed via `extra` so that callers are
/// never locked out of their store's features.
use serde::Deserialize;

use super::sql::{PoolConfig, TlsConfig};

// ---------------------------------------------------------------------------
// KV provider enum
// ---------------------------------------------------------------------------

/// Supported key-value store providers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub enum KvProvider {
    /// Redis / Valkey.
    #[default]
    #[serde(alias = "redis")]
    Redis,
    /// etcd v3.
    #[serde(alias = "etcd")]
    Etcd,
    /// Amazon DynamoDB (single-table design).
    #[serde(alias = "dynamodb")]
    DynamoDB,
    /// Memcached.
    #[serde(alias = "memcached")]
    Memcached,
    /// Any other provider — value is the provider name string.
    #[serde(untagged)]
    Other(String),
}

// ---------------------------------------------------------------------------
// Single KV instance
// ---------------------------------------------------------------------------

/// A single key-value store instance configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct KvInstanceConfig {
    /// Human-readable name for this instance (used in logs / metrics).
    #[serde(default)]
    pub name: Option<String>,

    /// Connection URL (e.g. `redis://localhost:6379`).
    pub url: String,

    /// Connection pool settings.
    #[serde(default)]
    pub pool: PoolConfig,

    /// TLS / SSL settings.
    #[serde(default)]
    pub tls: TlsConfig,
}

// ---------------------------------------------------------------------------
// Redis-specific settings
// ---------------------------------------------------------------------------

/// Redis / Valkey specific configuration knobs.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RedisConfig {
    /// Database number (0-15).
    #[serde(default)]
    pub database: u8,

    /// Redis cluster mode.
    #[serde(default)]
    pub cluster: bool,

    /// Redis Sentinel configuration.
    #[serde(default)]
    pub sentinel: Option<RedisSentinelConfig>,

    /// Username for ACL-based authentication (Redis 6+).
    #[serde(default)]
    pub username: Option<String>,
}

/// Redis Sentinel failover configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RedisSentinelConfig {
    /// Sentinel master name.
    pub master_name: String,

    /// Sentinel node URLs.
    pub nodes: Vec<String>,
}

// ---------------------------------------------------------------------------
// Top-level KV config
// ---------------------------------------------------------------------------

/// Complete key-value backend configuration.
///
/// Supports enterprise topologies:
/// - **Single instance**: set `primary` only.
/// - **Replicas**: add `replicas` for read scaling.
/// - **Named instances**: use `instances` for isolated workloads
///   (sessions, cache, rate-limiting, etc.).
#[derive(Debug, Clone, Deserialize)]
pub struct KvConfig {
    /// Which KV provider to use.
    #[serde(default)]
    pub provider: KvProvider,

    /// Primary (read-write) instance.
    pub primary: KvInstanceConfig,

    /// Read-replica instances.
    #[serde(default)]
    pub replicas: Vec<KvInstanceConfig>,

    /// Additional named instances.
    #[serde(default)]
    pub instances: std::collections::HashMap<String, KvInstanceConfig>,

    /// Key prefix applied to all operations.
    #[serde(default)]
    pub key_prefix: Option<String>,

    /// Default TTL (seconds) for entries without an explicit TTL.
    #[serde(default)]
    pub default_ttl_secs: Option<u64>,

    /// Redis-specific settings (only used when `provider` is `Redis`).
    #[serde(default)]
    pub redis: Option<RedisConfig>,

    /// Arbitrary extra settings for any provider.
    ///
    /// This catch-all ensures callers are never limited by the config schema.
    #[serde(default)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kv_config_single_redis() {
        let toml_str = r#"
            provider = "Redis"
            key_prefix = "app:"
            default_ttl_secs = 3600

            [primary]
            url = "redis://localhost:6379"
        "#;
        let cfg: KvConfig = toml_crate::from_str(toml_str).unwrap();
        assert_eq!(cfg.provider, KvProvider::Redis);
        assert_eq!(cfg.primary.url, "redis://localhost:6379");
        assert_eq!(cfg.key_prefix.as_deref(), Some("app:"));
        assert_eq!(cfg.default_ttl_secs, Some(3600));
        assert!(cfg.replicas.is_empty());
    }

    #[test]
    fn kv_config_with_replicas() {
        let toml_str = r#"
            provider = "Redis"

            [primary]
            url = "redis://primary:6379"

            [[replicas]]
            name = "replica-1"
            url = "redis://replica-1:6379"

            [[replicas]]
            name = "replica-2"
            url = "redis://replica-2:6379"
        "#;
        let cfg: KvConfig = toml_crate::from_str(toml_str).unwrap();
        assert_eq!(cfg.replicas.len(), 2);
    }

    #[test]
    fn kv_config_named_instances() {
        let toml_str = r#"
            provider = "Redis"

            [primary]
            url = "redis://localhost:6379"

            [instances.sessions]
            url = "redis://sessions-host:6379"

            [instances.cache]
            url = "redis://cache-host:6379"

            [instances.cache.pool]
            max_connections = 50
        "#;
        let cfg: KvConfig = toml_crate::from_str(toml_str).unwrap();
        assert!(cfg.instances.contains_key("sessions"));
        assert!(cfg.instances.contains_key("cache"));
        assert_eq!(cfg.instances["cache"].pool.max_connections, 50);
    }

    #[test]
    fn kv_config_with_sentinel() {
        let toml_str = r#"
            provider = "Redis"

            [primary]
            url = "redis://localhost:6379"

            [redis]
            cluster = false

            [redis.sentinel]
            master_name = "mymaster"
            nodes = ["redis://sentinel-1:26379", "redis://sentinel-2:26379"]
        "#;
        let cfg: KvConfig = toml_crate::from_str(toml_str).unwrap();
        let redis = cfg.redis.unwrap();
        let sentinel = redis.sentinel.unwrap();
        assert_eq!(sentinel.master_name, "mymaster");
        assert_eq!(sentinel.nodes.len(), 2);
    }

    #[test]
    fn kv_config_with_tls() {
        let toml_str = r#"
            provider = "Redis"

            [primary]
            url = "rediss://host:6380"

            [primary.tls]
            enabled = true
            ca_cert_path = "/etc/ssl/redis-ca.pem"
        "#;
        let cfg: KvConfig = toml_crate::from_str(toml_str).unwrap();
        assert!(cfg.primary.tls.enabled);
    }
}
