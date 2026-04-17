use super::*;
use dol_query::builder::storage::{GetObjectBuilder, ListObjectsBuilder, PutObjectBuilder};

#[test]
fn put_object_renders() {
    let ir = PutObjectBuilder::new("img/logo.png")
        .into_bucket("assets")
        .content_type("image/png")
        .build();
    let stmt = Statement::PutObject(ir);
    let result = ObjectStorageBackend.render(&stmt).unwrap();
    match result {
        RenderedOutput::Storage(out) => match out.operation {
            StorageOp::PutObject {
                bucket,
                key,
                content_type,
            } => {
                assert_eq!(bucket, "assets");
                assert_eq!(key, "img/logo.png");
                assert_eq!(content_type, Some("image/png".to_string()));
            }
            _ => panic!("expected PutObject"),
        },
        _ => panic!("expected Storage output"),
    }
}

#[test]
fn get_object_renders() {
    let ir = GetObjectBuilder::new("img/logo.png")
        .from_bucket("assets")
        .build();
    let stmt = Statement::GetObject(ir);
    let result = ObjectStorageBackend.render(&stmt).unwrap();
    match result {
        RenderedOutput::Storage(out) => match out.operation {
            StorageOp::GetObject { bucket, key } => {
                assert_eq!(bucket, "assets");
                assert_eq!(key, "img/logo.png");
            }
            _ => panic!("expected GetObject"),
        },
        _ => panic!("expected Storage output"),
    }
}

#[test]
fn list_objects_renders() {
    let ir = ListObjectsBuilder::new()
        .bucket("assets")
        .prefix("img/")
        .limit(10)
        .build();
    let stmt = Statement::ListObjects(ir);
    let result = ObjectStorageBackend.render(&stmt).unwrap();
    match result {
        RenderedOutput::Storage(out) => match out.operation {
            StorageOp::ListObjects {
                bucket,
                prefix,
                limit,
            } => {
                assert_eq!(bucket, "assets");
                assert_eq!(prefix, Some("img/".to_string()));
                assert_eq!(limit, Some(10));
            }
            _ => panic!("expected ListObjects"),
        },
        _ => panic!("expected Storage output"),
    }
}

#[test]
fn non_storage_statement_unsupported() {
    let stmt = Statement::Transaction(dol_core::ir::TransactionIR::Begin);
    let result = ObjectStorageBackend.render(&stmt);
    assert!(result.is_err());
}
