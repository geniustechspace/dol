use super::*;
use dol_core::ir::{Statement, TransactionIR};
use dol_core::ir::storage::{GetObjectIR, ListObjectsIR, PutObjectIR, ObjectSource};

#[test]
fn put_object_renders() {
    let stmt = Statement::PutObject(PutObjectIR { key: "img/logo.png".to_string(), source: ObjectSource::FromBytes, bucket: "assets".to_string(), content_type: Some("image/png".to_string()), metadata: vec![] });
    let result = ObjectStorageBackend.render(&stmt).unwrap();
    match result.operation { StorageOp::PutObject { bucket, key, content_type } => { assert_eq!(bucket, "assets"); assert_eq!(key, "img/logo.png"); assert_eq!(content_type, Some("image/png".to_string())); } _ => panic!("expected PutObject") }
}
#[test]
fn get_object_renders() {
    let stmt = Statement::GetObject(GetObjectIR { key: "img/logo.png".to_string(), bucket: "assets".to_string() });
    let result = ObjectStorageBackend.render(&stmt).unwrap();
    match result.operation { StorageOp::GetObject { bucket, key } => { assert_eq!(bucket, "assets"); assert_eq!(key, "img/logo.png"); } _ => panic!("expected GetObject") }
}
#[test]
fn list_objects_renders() {
    let stmt = Statement::ListObjects(ListObjectsIR { bucket: "assets".to_string(), prefix: Some("img/".to_string()), limit: Some(10), continuation_token: None });
    let result = ObjectStorageBackend.render(&stmt).unwrap();
    match result.operation { StorageOp::ListObjects { bucket, prefix, limit } => { assert_eq!(bucket, "assets"); assert_eq!(prefix, Some("img/".to_string())); assert_eq!(limit, Some(10)); } _ => panic!("expected ListObjects") }
}
#[test]
fn non_storage_statement_unsupported() {
    let stmt = Statement::Transaction(TransactionIR::Begin);
    assert!(ObjectStorageBackend.render(&stmt).is_err());
}
