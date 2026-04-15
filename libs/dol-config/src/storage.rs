/// Object / blob storage backend configuration with multi-instance support.
///
/// Covers S3-compatible stores (AWS S3, MinIO, GCS with S3 interop),
/// Google Cloud Storage, Azure Blob, and local filesystem.
/// Provider-specific settings are exposed via dedicated sub-structs and
/// an `extra` catch-all so callers are never limited.
use serde::{Deserialize, Serialize};

use super::sql::TlsConfig;

// ---------------------------------------------------------------------------
// Storage provider enum
// ---------------------------------------------------------------------------

/// Supported object / blob storage providers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageProvider {
    /// AWS S3 or any S3-compatible store (MinIO, DigitalOcean Spaces, etc.).
    #[default]
    #[serde(alias = "s3")]
    S3,
    /// Google Cloud Storage.
    #[serde(alias = "gcs")]
    Gcs,
    /// Azure Blob Storage.
    #[serde(alias = "azure", alias = "azure-blob")]
    AzureBlob,
    /// Local filesystem (for development / testing).
    #[serde(alias = "local", alias = "filesystem")]
    Local,
    /// Any other provider — value is the provider name string.
    #[serde(untagged)]
    Other(String),
}

// ---------------------------------------------------------------------------
// Single storage instance
// ---------------------------------------------------------------------------

/// A single object storage instance configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInstanceConfig {
    /// Human-readable name for this instance (used in logs / metrics).
    #[serde(default)]
    pub name: Option<String>,

    /// Default bucket / container name.
    pub bucket: String,

    /// Cloud region (e.g. `us-east-1`, `europe-west1`).
    #[serde(default)]
    pub region: Option<String>,

    /// Custom endpoint URL (for MinIO, LocalStack, S3-compatible stores).
    #[serde(default)]
    pub endpoint: Option<String>,

    /// Force path-style URLs (`http://host/bucket/key` instead of `http://bucket.host/key`).
    ///
    /// Required for most S3-compatible stores (MinIO, LocalStack).
    #[serde(default)]
    pub force_path_style: bool,

    /// TLS / SSL settings.
    #[serde(default)]
    pub tls: TlsConfig,
}

// ---------------------------------------------------------------------------
// S3-specific settings
// ---------------------------------------------------------------------------

/// AWS S3 specific configuration knobs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct S3Config {
    /// Access key ID (leave empty to use environment / instance profile).
    #[serde(default)]
    pub access_key_id: Option<String>,

    /// Secret access key (leave empty to use environment / instance profile).
    #[serde(default)]
    pub secret_access_key: Option<String>,

    /// Session token for temporary credentials (STS).
    #[serde(default)]
    pub session_token: Option<String>,

    /// IAM role ARN to assume via STS.
    #[serde(default)]
    pub role_arn: Option<String>,

    /// S3 storage class (e.g. `STANDARD`, `INTELLIGENT_TIERING`, `GLACIER`).
    #[serde(default)]
    pub storage_class: Option<String>,

    /// Server-side encryption method (`AES256`, `aws:kms`).
    #[serde(default)]
    pub server_side_encryption: Option<String>,

    /// KMS key ID for SSE-KMS encryption.
    #[serde(default)]
    pub kms_key_id: Option<String>,
}

// ---------------------------------------------------------------------------
// GCS-specific settings
// ---------------------------------------------------------------------------

/// Google Cloud Storage specific configuration knobs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GcsConfig {
    /// Path to a service account JSON key file.
    #[serde(default)]
    pub service_account_key_path: Option<String>,

    /// GCS storage class (e.g. `STANDARD`, `NEARLINE`, `COLDLINE`, `ARCHIVE`).
    #[serde(default)]
    pub storage_class: Option<String>,
}

// ---------------------------------------------------------------------------
// Azure-specific settings
// ---------------------------------------------------------------------------

/// Azure Blob Storage specific configuration knobs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AzureBlobConfig {
    /// Azure storage account name.
    #[serde(default)]
    pub account_name: Option<String>,

    /// Azure storage account key.
    #[serde(default)]
    pub account_key: Option<String>,

    /// Connection string (alternative to account name + key).
    #[serde(default)]
    pub connection_string: Option<String>,

    /// Access tier (`Hot`, `Cool`, `Archive`).
    #[serde(default)]
    pub access_tier: Option<String>,
}

// ---------------------------------------------------------------------------
// Local filesystem settings
// ---------------------------------------------------------------------------

/// Local filesystem storage configuration (development / testing).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalStorageConfig {
    /// Base directory for stored objects.
    #[serde(default)]
    pub base_path: Option<String>,
}

// ---------------------------------------------------------------------------
// Top-level storage config
// ---------------------------------------------------------------------------

/// Complete object storage backend configuration.
///
/// Supports enterprise topologies:
/// - **Single instance**: set `primary` only.
/// - **Replicas**: add `replicas` for geo-redundancy or read scaling.
/// - **Named instances**: use `instances` for isolated workloads
///   (uploads, documents, backups, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Which storage provider to use.
    #[serde(default)]
    pub provider: StorageProvider,

    /// Primary storage instance.
    pub primary: StorageInstanceConfig,

    /// Replica storage instances.
    #[serde(default)]
    pub replicas: Vec<StorageInstanceConfig>,

    /// Additional named instances.
    #[serde(default)]
    pub instances: std::collections::HashMap<String, StorageInstanceConfig>,

    /// S3-specific settings (only used when `provider` is `S3`).
    #[serde(default)]
    pub s3: Option<S3Config>,

    /// GCS-specific settings (only used when `provider` is `Gcs`).
    #[serde(default)]
    pub gcs: Option<GcsConfig>,

    /// Azure Blob specific settings (only used when `provider` is `AzureBlob`).
    #[serde(default)]
    pub azure: Option<AzureBlobConfig>,

    /// Local filesystem settings (only used when `provider` is `Local`).
    #[serde(default)]
    pub local: Option<LocalStorageConfig>,

    /// Arbitrary extra settings for any provider.
    #[serde(default)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_config_single_s3() {
        let toml_str = r#"
            provider = "S3"

            [primary]
            bucket = "my-app-assets"
            region = "us-east-1"
        "#;
        let cfg: StorageConfig = toml_crate::from_str(toml_str).unwrap();
        assert_eq!(cfg.provider, StorageProvider::S3);
        assert_eq!(cfg.primary.bucket, "my-app-assets");
        assert_eq!(cfg.primary.region.as_deref(), Some("us-east-1"));
    }

    #[test]
    fn storage_config_minio() {
        let toml_str = r#"
            provider = "S3"

            [primary]
            bucket = "dev-bucket"
            endpoint = "http://localhost:9000"
            force_path_style = true

            [s3]
            access_key_id = "minioadmin"
            secret_access_key = "minioadmin"
        "#;
        let cfg: StorageConfig = toml_crate::from_str(toml_str).unwrap();
        assert_eq!(
            cfg.primary.endpoint.as_deref(),
            Some("http://localhost:9000")
        );
        assert!(cfg.primary.force_path_style);
        let s3 = cfg.s3.unwrap();
        assert_eq!(s3.access_key_id.as_deref(), Some("minioadmin"));
    }

    #[test]
    fn storage_config_with_replicas() {
        let toml_str = r#"
            provider = "S3"

            [primary]
            bucket = "primary-bucket"
            region = "us-east-1"

            [[replicas]]
            name = "eu-replica"
            bucket = "eu-bucket"
            region = "eu-west-1"

            [[replicas]]
            name = "ap-replica"
            bucket = "ap-bucket"
            region = "ap-southeast-1"
        "#;
        let cfg: StorageConfig = toml_crate::from_str(toml_str).unwrap();
        assert_eq!(cfg.replicas.len(), 2);
    }

    #[test]
    fn storage_config_named_instances() {
        let toml_str = r#"
            provider = "S3"

            [primary]
            bucket = "main-bucket"

            [instances.uploads]
            bucket = "uploads-bucket"
            region = "us-east-1"

            [instances.backups]
            bucket = "backups-bucket"
            region = "us-west-2"
        "#;
        let cfg: StorageConfig = toml_crate::from_str(toml_str).unwrap();
        assert!(cfg.instances.contains_key("uploads"));
        assert!(cfg.instances.contains_key("backups"));
    }

    #[test]
    fn storage_config_local_provider() {
        let toml_str = r#"
            provider = "Local"

            [primary]
            bucket = "local-data"

            [local]
            base_path = "/tmp/dol-storage"
        "#;
        let cfg: StorageConfig = toml_crate::from_str(toml_str).unwrap();
        assert_eq!(cfg.provider, StorageProvider::Local);
        let local = cfg.local.unwrap();
        assert_eq!(local.base_path.as_deref(), Some("/tmp/dol-storage"));
    }

    #[test]
    fn storage_config_azure() {
        let toml_str = r#"
            provider = "AzureBlob"

            [primary]
            bucket = "my-container"

            [azure]
            account_name = "mystorage"
            access_tier = "Hot"
        "#;
        let cfg: StorageConfig = toml_crate::from_str(toml_str).unwrap();
        assert_eq!(cfg.provider, StorageProvider::AzureBlob);
        let az = cfg.azure.unwrap();
        assert_eq!(az.account_name.as_deref(), Some("mystorage"));
    }
}
