//! Storage operations — canonical representation of object/file storage operations.

use crate::expr::Expr;

/// Put (upload/write) an object into a bucket/store.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PutObject<'a> {
    pub key: String,
    pub source: ObjectSource<'a>,
    pub bucket: String,
    pub content_type: Option<String>,
    pub metadata: Vec<(String, String)>,
}

/// Source of data for a storage operation.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ObjectSource<'a> {
    /// From a file path.
    FromPath(String),
    /// From a bind parameter (bytes).
    FromBytes,
    /// From an expression.
    FromExpr(Expr<'a>),
}

/// Get (download/read) an object from a bucket/store.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GetObject {
    pub key: String,
    pub bucket: String,
}

/// List objects in a bucket/store.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ListObjects {
    pub bucket: String,
    pub prefix: Option<String>,
    pub limit: Option<u64>,
    pub continuation_token: Option<String>,
}

/// Read a file from the filesystem.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ReadFile {
    pub path: String,
    pub encoding: Option<String>,
}

/// Write content to a file.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WriteFile<'a> {
    pub path: String,
    pub source: ObjectSource<'a>,
    pub create_dirs: bool,
}

/// Move/rename a file.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MoveFile {
    pub from: String,
    pub to: String,
}
