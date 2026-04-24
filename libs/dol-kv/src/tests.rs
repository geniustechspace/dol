use super::*;
use dol_ir::statement::Statement;
use dol_ir::transaction::Transaction;
use dol_ir::storage::{GetObject, ObjectSource, PutObject};

#[test]
fn put_object_as_kv() {
    let ir = PutObject {
        key:          "key/path".to_string(),
        source:       ObjectSource::FromBytes,
        bucket:       "bucket".to_string(),
        content_type: None,
        metadata:     vec![],
    };
    let stmt = Statement::PutObject(ir);
    let result = KvBackend.render(&stmt).unwrap();
    assert_eq!(result.operation, KvOp::Put);
    assert_eq!(result.key, "bucket/key/path");
}

#[test]
fn get_object_as_kv() {
    let ir = GetObject {
        key:    "key/path".to_string(),
        bucket: "bucket".to_string(),
    };
    let stmt = Statement::GetObject(ir);
    let result = KvBackend.render(&stmt).unwrap();
    assert_eq!(result.operation, KvOp::Get);
    assert_eq!(result.key, "bucket/key/path");
}

#[test]
fn sql_statement_unsupported() {
    let stmt = Statement::Transaction(Transaction::Begin);
    assert!(KvBackend.render(&stmt).is_err());
}
