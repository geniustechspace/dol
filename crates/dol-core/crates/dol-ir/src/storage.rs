//! Storage IR — canonical representation of object/file storage operations.

use dol_expr::Expr;

/// Put (upload/write) an object into a bucket/store.
#[derive(Debug, Clone)]
pub struct PutObjectIR {
    pub key: String,
    pub source: ObjectSource,
    pub bucket: String,
    pub content_type: Option<String>,
    pub metadata: Vec<(String, String)>,
}

/// Source of data for a storage operation.
#[derive(Debug, Clone)]
pub enum ObjectSource {
    /// From a file path.
    FromPath(String),
    /// From a bind parameter (bytes).
    FromBytes,
    /// From an expression.
    FromExpr(Expr),
}

/// Get (download/read) an object from a bucket/store.
#[derive(Debug, Clone)]
pub struct GetObjectIR {
    pub key: String,
    pub bucket: String,
}

/// List objects in a bucket/store.
#[derive(Debug, Clone)]
pub struct ListObjectsIR {
    pub bucket: String,
    pub prefix: Option<String>,
    pub limit: Option<u64>,
    pub continuation_token: Option<String>,
}

/// Read a file from the filesystem.
#[derive(Debug, Clone)]
pub struct ReadFileIR {
    pub path: String,
    pub encoding: Option<String>,
}

/// Write content to a file.
#[derive(Debug, Clone)]
pub struct WriteFileIR {
    pub path: String,
    pub source: ObjectSource,
    pub create_dirs: bool,
}

/// Move/rename a file.
#[derive(Debug, Clone)]
pub struct MoveFileIR {
    pub from: String,
    pub to: String,
}
