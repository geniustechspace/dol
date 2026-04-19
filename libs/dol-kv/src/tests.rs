use super::*;
use dol_core::ir::{Statement, TransactionIR};
use dol_core::ir::storage::{GetObjectIR, PutObjectIR, ObjectSource};

#[test]
fn put_object_as_kv() {
    let ir = PutObjectIR {
        key: "key/path".to_string(),
        source: ObjectSource::FromBytes,
        bucket: "bucket".to_string(),
        content_type: None,
        metadata: vec![],
    };
    let stmt = Statement::PutObject(ir);
    let result = KvBackend.render(&stmt).unwrap();
    assert_eq!(result.operation, KvOp::Put);
    assert_eq!(result.key, "bucket/key/path");
}

#[test]
fn get_object_as_kv() {
    let ir = GetObjectIR {
        key: "key/path".to_string(),
        bucket: "bucket".to_string(),
    };
    let stmt = Statement::GetObject(ir);
    let result = KvBackend.render(&stmt).unwrap();
    assert_eq!(result.operation, KvOp::Get);
    assert_eq!(result.key, "bucket/key/path");
}

#[test]
fn sql_statement_unsupported() {
    let stmt = Statement::Transaction(TransactionIR::Begin);
    assert!(KvBackend.render(&stmt).is_err());
}
