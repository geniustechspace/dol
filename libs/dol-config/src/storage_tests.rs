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
