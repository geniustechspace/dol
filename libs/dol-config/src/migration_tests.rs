use super::*;

#[test]
fn default_config() {
    let cfg = UnifiedMigrationConfig::default();
    assert!(!cfg.auto_run);
    assert_eq!(cfg.table_name, "_dol_migrations");
    assert!(cfg.backends.sql);
    assert!(cfg.backends.kv);
    assert!(cfg.backends.storage);
    assert!(cfg.validate_checksums);
    assert!(cfg.abort_on_error);
    assert!(!cfg.dry_run);
}

#[test]
fn deserialize_from_toml() {
    let toml_str = r#"
        auto_run = true
        table_name = "schema_versions"
        dry_run = true
        lock_strategy = "advisory_lock"

        [backends]
        sql = true
        kv = false
        storage = false
    "#;
    let cfg: UnifiedMigrationConfig = toml_crate::from_str(toml_str).unwrap();
    assert!(cfg.auto_run);
    assert_eq!(cfg.table_name, "schema_versions");
    assert!(cfg.backends.sql);
    assert!(!cfg.backends.kv);
    assert!(!cfg.backends.storage);
    assert!(cfg.dry_run);
    assert!(matches!(cfg.lock_strategy, LockStrategy::AdvisoryLock));
}

#[test]
fn deserialize_minimal() {
    let toml_str = "auto_run = true";
    let cfg: UnifiedMigrationConfig = toml_crate::from_str(toml_str).unwrap();
    assert!(cfg.auto_run);
    assert_eq!(cfg.table_name, "_dol_migrations");
}

#[test]
fn lock_strategies() {
    let cases = [
        (r#"lock_strategy = "none""#, "none"),
        (r#"lock_strategy = "advisory_lock""#, "advisory_lock"),
        (r#"lock_strategy = "lock_table""#, "lock_table"),
    ];
    for (toml_str, _label) in cases {
        let cfg: UnifiedMigrationConfig = toml_crate::from_str(toml_str).unwrap();
        // Just ensure it deserializes without error
        let _ = cfg.lock_strategy;
    }
}
