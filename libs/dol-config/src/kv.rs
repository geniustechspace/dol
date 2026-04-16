/// Key-value backend configuration with multi-instance (primary, replicas) support.
///
/// Covers Redis, etcd, DynamoDB, Memcached and other KV providers.
/// All provider-specific knobs are exposed via `extra` so that callers are
/// never locked out of their store's features.
use serde::{Deserialize, Serialize};

use super::sql::{PoolConfig, TlsConfig};

// ---------------------------------------------------------------------------
// KV provider enum
// ---------------------------------------------------------------------------

/// Supported key-value store providers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[path = "kv_tests.rs"]
mod tests;
