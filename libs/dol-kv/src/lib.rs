//! # dol-kv — DOL Key-Value Backend

#![deny(unsafe_code)]

use dol_core::ir::Statement;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct KvOutput {
    pub operation: KvOp,
    pub key: String,
    pub metadata: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum KvOp {
    Get,
    Put,
    Delete,
    List { prefix: Option<String>, limit: Option<u64> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KvError {
    Unsupported(String),
}

impl std::fmt::Display for KvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(msg) => write!(f, "unsupported: {}", msg),
        }
    }
}

impl std::error::Error for KvError {}

pub struct KvBackend;

impl KvBackend {
    pub fn render(&self, stmt: &Statement) -> Result<KvOutput, KvError> {
        match stmt {
            Statement::PutObject(ir) => Ok(KvOutput {
                operation: KvOp::Put,
                key: format!("{}/{}", ir.bucket, ir.key),
                metadata: ir.metadata.clone(),
            }),
            Statement::GetObject(ir) => Ok(KvOutput {
                operation: KvOp::Get,
                key: format!("{}/{}", ir.bucket, ir.key),
                metadata: vec![],
            }),
            Statement::ListObjects(ir) => Ok(KvOutput {
                operation: KvOp::List {
                    prefix: ir.prefix.clone(),
                    limit: ir.limit,
                },
                key: ir.bucket.clone(),
                metadata: vec![],
            }),
            Statement::ReadFile(ir) => Ok(KvOutput {
                operation: KvOp::Get,
                key: ir.path.clone(),
                metadata: vec![],
            }),
            Statement::WriteFile(ir) => Ok(KvOutput {
                operation: KvOp::Put,
                key: ir.path.clone(),
                metadata: vec![],
            }),
            Statement::MoveFile(ir) => Ok(KvOutput {
                operation: KvOp::Put,
                key: ir.to.clone(),
                metadata: vec![("moved_from".into(), ir.from.clone())],
            }),
            _ => Err(KvError::Unsupported(
                "KvBackend only supports storage and file operations".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests;
