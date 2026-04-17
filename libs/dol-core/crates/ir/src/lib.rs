//! # dol-ir — DOL Intermediate Representation
//!
//! The canonical, backend-agnostic AST that all DOL builders produce and
//! all backends consume.
//!
//! Also defines the `Backend` trait, output types, and error types shared
//! across all backend implementations.

#![deny(unsafe_code)]

pub mod control;
pub mod definition;
pub mod mutation;
pub mod query;
pub mod storage;
pub mod transaction;

pub use control::{DefinePolicyIR, GrantIR, PolicyAction, Privilege, RevokeIR};
pub use definition::{
    AlterAction, AlterEntityIR, DefineEntityIR, DefineIndexIR, DefineTypeIR, DropEntityIR,
    DropIndexIR, DropTypeIR, FieldDef, IndexMethod, OwnedEntityConstraint, OwnedForeignKeyRef,
};
pub use mutation::{InsertIR, InsertSelectIR, RemoveIR, UpdateIR, UpsertIR};
pub use query::{CompoundQueryIR, JoinIR, JoinType, LockMode, OffsetLimit, QueryIR, SetOpKind};
pub use storage::{
    GetObjectIR, ListObjectsIR, MoveFileIR, ObjectSource, PutObjectIR, ReadFileIR, WriteFileIR,
};
pub use transaction::TransactionIR;

/// A reference to a model (table/collection/bucket), with optional alias.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityRef {
    pub name: String,
    pub namespace: Option<String>,
    pub alias: Option<String>,
}

/// Top-level DOL statement — the universal dispatch enum.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Statement<'a> {
    // Definition
    DefineEntity(Box<DefineEntityIR>),
    AlterEntity(AlterEntityIR),
    DropEntity(DropEntityIR),
    DefineIndex(DefineIndexIR),
    DropIndex(DropIndexIR),
    DefineType(DefineTypeIR),
    DropType(DropTypeIR),

    // Mutation
    Insert(InsertIR),
    InsertSelect(InsertSelectIR),
    Update(UpdateIR<'a>),
    Remove(RemoveIR<'a>),
    Upsert(Box<UpsertIR<'a>>),

    // Query
    Query(Box<QueryIR<'a>>),

    // Compound query (set operations)
    Compound(Box<CompoundQueryIR<'a>>),

    // Control
    Grant(GrantIR),
    Revoke(RevokeIR),
    DefinePolicy(DefinePolicyIR<'a>),

    // Transaction
    Transaction(TransactionIR<'a>),

    // Storage
    PutObject(PutObjectIR<'a>),
    GetObject(GetObjectIR),
    ListObjects(ListObjectsIR),
    ReadFile(ReadFileIR),
    WriteFile(WriteFileIR<'a>),
    MoveFile(MoveFileIR),
}

// ---------------------------------------------------------------------------
// Backend trait + output types (shared across all backends)
// ---------------------------------------------------------------------------

/// SQL output (text + parameter count).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SqlOutput {
    pub sql: String,
    pub param_count: usize,
}

/// A key-value operation descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct KvOutput {
    /// The operation to perform.
    pub operation: KvOp,
    /// The key to operate on.
    pub key: String,
    /// Optional metadata (key-value pairs).
    pub metadata: Vec<(String, String)>,
}

/// Key-value operation kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum KvOp {
    Get,
    Put,
    Delete,
    List {
        prefix: Option<String>,
        limit: Option<u64>,
    },
}

/// An object storage operation descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StorageOutput {
    /// The operation to perform.
    pub operation: StorageOp,
}

/// Object storage operation kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StorageOp {
    PutObject {
        bucket: String,
        key: String,
        content_type: Option<String>,
    },
    GetObject {
        bucket: String,
        key: String,
    },
    ListObjects {
        bucket: String,
        prefix: Option<String>,
        limit: Option<u64>,
    },
    DeleteObject {
        bucket: String,
        key: String,
    },
}

/// The rendered output of a backend.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RenderedOutput {
    /// SQL text + parameter count.
    Sql(SqlOutput),
    /// Key-value operation descriptor.
    KeyValue(KvOutput),
    /// Object storage operation descriptor.
    Storage(StorageOutput),
}

/// Backend rendering errors.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BackendError {
    /// This backend does not support this statement kind.
    Unsupported(String),
    /// Rendering failed.
    RenderError(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(msg) => write!(f, "unsupported: {}", msg),
            Self::RenderError(msg) => write!(f, "render error: {}", msg),
        }
    }
}

impl std::error::Error for BackendError {}

/// The core backend trait. Each backend renders IR into its output format.
pub trait Backend {
    fn render(&self, stmt: &Statement<'_>) -> Result<RenderedOutput, BackendError>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
