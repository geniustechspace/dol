//! # dol-ir — DOL Intermediate Representation
//!
//! The canonical, backend-agnostic AST that all DOL builders produce and
//! all backends consume.
//!
//! Also defines the `Backend` trait, output types, and error types shared
//! across all backend implementations.

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

// ---------------------------------------------------------------------------
// BackendError — shared error type for all backends
// ---------------------------------------------------------------------------

/// Errors that can occur during backend rendering.
///
/// Every DOL backend (SQL, spreadsheet, KV, object-storage, …) uses this
/// shared type so callers can handle errors uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BackendError {
    /// The backend does not support the requested operation or expression.
    Unsupported(String),
    /// A rendering error that is not an unsupported-feature issue.
    Render(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(msg) => write!(f, "unsupported: {}", msg),
            Self::Render(msg) => write!(f, "render error: {}", msg),
        }
    }
}

impl std::error::Error for BackendError {}

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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
