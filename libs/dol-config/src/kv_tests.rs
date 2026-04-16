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
