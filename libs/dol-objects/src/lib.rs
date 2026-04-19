//! # dol-objects — DOL Object Storage Backend
#![deny(unsafe_code)]
use dol_core::ir::Statement;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StorageOutput { pub operation: StorageOp }

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StorageOp {
    PutObject { bucket: String, key: String, content_type: Option<String> },
    GetObject { bucket: String, key: String },
    ListObjects { bucket: String, prefix: Option<String>, limit: Option<u64> },
    DeleteObject { bucket: String, key: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError { Unsupported(String) }
impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::Unsupported(msg) => write!(f, "unsupported: {}", msg) }
    }
}
impl std::error::Error for StorageError {}

pub struct ObjectStorageBackend;
impl ObjectStorageBackend {
    pub fn render(&self, stmt: &Statement) -> Result<StorageOutput, StorageError> {
        match stmt {
            Statement::PutObject(ir) => Ok(StorageOutput { operation: StorageOp::PutObject { bucket: ir.bucket.clone(), key: ir.key.clone(), content_type: ir.content_type.clone() } }),
            Statement::GetObject(ir) => Ok(StorageOutput { operation: StorageOp::GetObject { bucket: ir.bucket.clone(), key: ir.key.clone() } }),
            Statement::ListObjects(ir) => Ok(StorageOutput { operation: StorageOp::ListObjects { bucket: ir.bucket.clone(), prefix: ir.prefix.clone(), limit: ir.limit } }),
            _ => Err(StorageError::Unsupported("ObjectStorageBackend only supports storage operations".into())),
        }
    }
}
#[cfg(test)]
mod tests;
