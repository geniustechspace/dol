use super::*;
use dol_core::builder::storage::{GetObjectBuilder, PutObjectBuilder};

#[test]
fn put_object_as_kv() {
    let ir = PutObjectBuilder::new("key/path")
        .into_bucket("bucket")
        .build();
    let stmt = Statement::PutObject(ir);
    let result = KvBackend.render(&stmt).unwrap();
    match result {
        RenderedOutput::KeyValue(out) => {
            assert_eq!(out.operation, KvOp::Put);
            assert_eq!(out.key, "bucket/key/path");
        }
        _ => panic!("expected KeyValue output"),
    }
}

#[test]
fn get_object_as_kv() {
    let ir = GetObjectBuilder::new("key/path")
        .from_bucket("bucket")
        .build();
    let stmt = Statement::GetObject(ir);
    let result = KvBackend.render(&stmt).unwrap();
    match result {
        RenderedOutput::KeyValue(out) => {
            assert_eq!(out.operation, KvOp::Get);
            assert_eq!(out.key, "bucket/key/path");
        }
        _ => panic!("expected KeyValue output"),
    }
}

#[test]
fn sql_statement_unsupported() {
    let stmt = Statement::Transaction(dol_core::ir::TransactionIR::Begin);
    assert!(KvBackend.render(&stmt).is_err());
}
