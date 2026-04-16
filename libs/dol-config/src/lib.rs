//! Unified configuration for the DOL crate.
//!
//! `DolConfig` is the single entry-point for configuring all DOL backends
//! (SQL, key-value, object storage) and their dialects from one file.
//!

#![deny(unsafe_code)]
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

use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;

// ---------------------------------------------------------------------------
// Unified DOL config
// ---------------------------------------------------------------------------

/// Complete DOL configuration — all backends in a single struct.
///
/// Every section is optional so you can configure only the backends your
/// application actually uses.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    Yaml(serde_norway::Error),
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

impl From<serde_norway::Error> for ConfigError {
    fn from(e: serde_norway::Error) -> Self {
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
    pub fn from_yaml_str(yaml: &str) -> Result<Self, serde_norway::Error> {
        serde_norway::from_str(yaml)
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
mod tests;
