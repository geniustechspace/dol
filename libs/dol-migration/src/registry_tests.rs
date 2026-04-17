use super::*;

#[test]
fn in_memory_registry_basic() {
    let mut reg = InMemoryRegistry::new();
    assert!(reg.applied().unwrap().is_empty());

    reg.mark_applied("V001", "First migration", "checksum1")
        .unwrap();
    reg.mark_applied("V002", "Second migration", "checksum2")
        .unwrap();

    let applied = reg.applied().unwrap();
    assert_eq!(applied.len(), 2);
    assert_eq!(applied[0].version, "V001");
    assert_eq!(applied[1].version, "V002");
}

#[test]
fn in_memory_registry_revert() {
    let mut reg = InMemoryRegistry::new();
    reg.mark_applied("V001", "First", "c1").unwrap();
    reg.mark_applied("V002", "Second", "c2").unwrap();

    reg.mark_reverted("V001").unwrap();
    let applied = reg.applied().unwrap();
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].version, "V002");
}

#[test]
fn in_memory_registry_duplicate() {
    let mut reg = InMemoryRegistry::new();
    reg.mark_applied("V001", "First", "c1").unwrap();
    let result = reg.mark_applied("V001", "Duplicate", "c2");
    assert!(matches!(result, Err(MigrationError::DuplicateVersion(_))));
}

#[test]
fn in_memory_registry_ordered() {
    let mut reg = InMemoryRegistry::new();
    // Insert out of order
    reg.mark_applied("V003", "Third", "c3").unwrap();
    reg.mark_applied("V001", "First", "c1").unwrap();
    reg.mark_applied("V002", "Second", "c2").unwrap();

    let applied = reg.applied().unwrap();
    assert_eq!(applied[0].version, "V001");
    assert_eq!(applied[1].version, "V002");
    assert_eq!(applied[2].version, "V003");
}

#[test]
fn migration_history_entity() {
    let hist = migration_history();
    assert_eq!(hist.name, "_dol_migrations");
    assert_eq!(hist.fields.len(), 5);
    assert!(hist.field("version").primary_key);
}

#[test]
fn sql_helpers_postgres() {
    let pg = dol_sql::dialect::Dialect::postgres();

    let create_sql = create_history_table_sql(Some(&pg));
    assert!(create_sql.contains("CREATE TABLE IF NOT EXISTS _dol_migrations"));
    assert!(create_sql.contains("version VARCHAR(255) NOT NULL"));
    assert!(create_sql.contains("checksum TEXT NOT NULL"));

    let insert_sql = insert_applied_sql(Some(&pg));
    assert!(insert_sql.contains("INSERT INTO _dol_migrations"));
    assert!(insert_sql.contains("$1"));

    let delete_sql = delete_reverted_sql(Some(&pg));
    assert!(delete_sql.contains("DELETE FROM _dol_migrations"));
    assert!(delete_sql.contains("$1"));

    let select_sql = select_applied_sql(Some(&pg));
    assert!(select_sql.contains("SELECT"));
    assert!(select_sql.contains("_dol_migrations"));
}

#[test]
fn sql_helpers_sqlite() {
    let create_sql = create_history_table_sql(None); // default is SQLite
    assert!(create_sql.contains("CREATE TABLE IF NOT EXISTS _dol_migrations"));

    let insert_sql = insert_applied_sql(None);
    assert!(insert_sql.contains("?"));

    let delete_sql = delete_reverted_sql(None);
    assert!(delete_sql.contains("?"));
}
