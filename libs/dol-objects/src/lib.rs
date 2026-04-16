//! # dol-objects — DOL Object Storage Backend
//!
//! Renders DOL IR into storage operation descriptors.

#![deny(unsafe_code)]

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
mod tests;
