//! # dol-objects — DOL Object Storage Backend
//!
//! Renders DOL IR into storage operation descriptors.

use dol_core::ir::{Backend, BackendError, RenderedOutput, Statement, StorageOp, StorageOutput};

/// Backend that renders storage IR statements into [`StorageOutput`] descriptors.
///
/// Supports `PutObject`, `GetObject`, and `ListObjects` statements.
/// Returns [`BackendError::Unsupported`] for SQL or other non-storage statements.
pub struct ObjectStorageBackend;

impl Backend for ObjectStorageBackend {
    fn render(&self, stmt: &Statement) -> Result<RenderedOutput, BackendError> {
        match stmt {
            Statement::PutObject(ir) => Ok(RenderedOutput::Storage(StorageOutput {
                operation: StorageOp::PutObject {
                    bucket: ir.bucket.clone(),
                    key: ir.key.clone(),
                    content_type: ir.content_type.clone(),
                },
            })),
            Statement::GetObject(ir) => Ok(RenderedOutput::Storage(StorageOutput {
                operation: StorageOp::GetObject {
                    bucket: ir.bucket.clone(),
                    key: ir.key.clone(),
                },
            })),
            Statement::ListObjects(ir) => Ok(RenderedOutput::Storage(StorageOutput {
                operation: StorageOp::ListObjects {
                    bucket: ir.bucket.clone(),
                    prefix: ir.prefix.clone(),
                    limit: ir.limit,
                },
            })),
            _ => Err(BackendError::Unsupported(
                "ObjectStorageBackend only supports storage operations".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dol_core::builder::storage::{GetObjectBuilder, ListObjectsBuilder, PutObjectBuilder};

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
}
