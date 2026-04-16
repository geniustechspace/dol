//! # dol-kv — DOL Key-Value Backend
//!
//! Renders DOL IR into key-value operation descriptors.

#![deny(unsafe_code)]

use dol_core::ir::{Backend, BackendError, KvOp, KvOutput, RenderedOutput, Statement};

/// Backend that renders storage IR statements into [`KvOutput`] descriptors.
///
/// Maps object storage operations to key-value semantics:
/// - `PutObject` → `KvOp::Put` with key = `bucket/key`
/// - `GetObject` → `KvOp::Get`
/// - `ListObjects` → `KvOp::List`
///
/// Returns [`BackendError::Unsupported`] for SQL-specific statements.
pub struct KvBackend;

impl Backend for KvBackend {
    fn render(&self, stmt: &Statement) -> Result<RenderedOutput, BackendError> {
        match stmt {
            Statement::PutObject(ir) => Ok(RenderedOutput::KeyValue(KvOutput {
                operation: KvOp::Put,
                key: format!("{}/{}", ir.bucket, ir.key),
                metadata: ir.metadata.clone(),
            })),
            Statement::GetObject(ir) => Ok(RenderedOutput::KeyValue(KvOutput {
                operation: KvOp::Get,
                key: format!("{}/{}", ir.bucket, ir.key),
                metadata: vec![],
            })),
            Statement::ListObjects(ir) => Ok(RenderedOutput::KeyValue(KvOutput {
                operation: KvOp::List {
                    prefix: ir.prefix.clone(),
                    limit: ir.limit,
                },
                key: ir.bucket.clone(),
                metadata: vec![],
            })),
            Statement::ReadFile(ir) => Ok(RenderedOutput::KeyValue(KvOutput {
                operation: KvOp::Get,
                key: ir.path.clone(),
                metadata: vec![],
            })),
            Statement::WriteFile(ir) => Ok(RenderedOutput::KeyValue(KvOutput {
                operation: KvOp::Put,
                key: ir.path.clone(),
                metadata: vec![],
            })),
            Statement::MoveFile(ir) => {
                // KV doesn't have a native move; model as delete + put metadata
                Ok(RenderedOutput::KeyValue(KvOutput {
                    operation: KvOp::Put,
                    key: ir.to.clone(),
                    metadata: vec![("moved_from".into(), ir.from.clone())],
                }))
            }
            _ => Err(BackendError::Unsupported(
                "KvBackend only supports storage and file operations".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests;
