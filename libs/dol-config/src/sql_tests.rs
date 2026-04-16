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
