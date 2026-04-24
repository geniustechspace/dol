//! Storage operations — object store and file system commands.

use dol_expr::ids::NodeId;

/// Source of data for a PUT or WRITE operation.
///
/// The `FromExpr` variant uses a [`NodeId`] arena reference instead of the
/// old `Expr<'a>` tree so `ObjectSource` has no lifetime parameter.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ObjectSource {
    /// Verbatim byte payload supplied by the caller at execution time.
    FromBytes,
    /// Filesystem path on the server.
    FromPath(String),
    /// Value of an arena expression evaluated at execution time.
    FromExpr(NodeId),
}

/// Upload / write an object to object storage.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PutObject {
    pub bucket: String,
    pub key: String,
    pub source: ObjectSource,
    pub content_type: Option<String>,
    pub metadata: Vec<(String, String)>,
}

/// Download / read an object from object storage.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GetObject {
    pub bucket: String,
    pub key: String,
}

/// List objects in a bucket (with optional prefix filter).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ListObjects {
    pub bucket: String,
    pub prefix: Option<String>,
    pub limit: Option<u64>,
    pub continuation_token: Option<String>,
}

/// Read a file from the local / remote file system.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ReadFile {
    pub path: String,
    pub encoding: Option<String>,
}

/// Write a file to the local / remote file system.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WriteFile {
    pub path: String,
    pub source: ObjectSource,
    pub create_dirs: bool,
}

/// Move / rename a file.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MoveFile {
    pub from: String,
    pub to: String,
}
