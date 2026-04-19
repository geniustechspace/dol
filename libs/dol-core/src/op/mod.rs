//! # dol-op — DOL Operations
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

pub use control::{DefinePolicy, Grant, PolicyAction, Privilege, Revoke};
pub use definition::{
    AlterAction, AlterEntity, DefineEntity, DefineIndex, DefineType, DropEntity,
    DropIndex, DropType, FieldDef, IndexMethod, OwnedEntityConstraint, OwnedForeignKeyRef,
};
pub use mutation::{Insert, InsertSelect, Remove, Update, Upsert};
pub use query::{CompoundQuery, Join, JoinKind, LockMode, OffsetLimit, Query, SetOp};
pub use storage::{
    GetObject, ListObjects, MoveFile, ObjectSource, PutObject, ReadFile, WriteFile,
};
pub use transaction::Transaction;

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
    DefineEntity(Box<DefineEntity>),
    AlterEntity(AlterEntity),
    DropEntity(DropEntity),
    DefineIndex(DefineIndex),
    DropIndex(DropIndex),
    DefineType(DefineType),
    DropType(DropType),

    // Mutation
    Insert(Insert),
    InsertSelect(InsertSelect),
    Update(Update<'a>),
    Remove(Remove<'a>),
    Upsert(Box<Upsert<'a>>),

    // Query
    Query(Box<Query<'a>>),

    // Compound query (set operations)
    Compound(Box<CompoundQuery<'a>>),

    // Control
    Grant(Grant),
    Revoke(Revoke),
    DefinePolicy(DefinePolicy<'a>),

    // Transaction
    Transaction(Transaction<'a>),

    // Storage
    PutObject(PutObject<'a>),
    GetObject(GetObject),
    ListObjects(ListObjects),
    ReadFile(ReadFile),
    WriteFile(WriteFile<'a>),
    MoveFile(MoveFile),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
