//! # dol-op — DOL Operations
//!
//! The canonical, backend-agnostic AST that all DOL builders produce and
//! all backends consume.
//!
//! Also defines the [`Operation`] trait for trait-based dispatch and the
//! [`Statement`] convenience enum.

pub mod control;
pub mod definition;
pub mod mutation;
pub mod query;
pub mod storage;
pub mod transaction;

pub use control::{DefinePolicy, Grant, PolicyAction, Privilege, Revoke};
pub use definition::{
    AlterAction, AlterEntity, Constraint, DefineEntity, DefineIndex, DefineType, DropEntity,
    DropIndex, DropType, FieldDef, ForeignKeyDef, IndexMethod, OwnedEntityConstraint,
    OwnedForeignKeyRef,
};
pub use mutation::{Insert, InsertSelect, Remove, Update, Upsert};
pub use query::{CompoundQuery, Join, JoinKind, LockMode, OffsetLimit, Query, SetOp};
pub use storage::{
    GetObject, ListObjects, MoveFile, ObjectSource, PutObject, ReadFile, WriteFile,
};
pub use transaction::Transaction;

use std::any::Any;
use std::fmt;

/// A reference to a model (table/collection/bucket), with optional alias.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityRef {
    pub name: String,
    pub namespace: Option<String>,
    pub alias: Option<String>,
}

// ---------------------------------------------------------------------------
// Operation trait — trait-based dispatch for DOL operations
// ---------------------------------------------------------------------------

/// Trait for DOL operations — the verb layer of the language.
///
/// Each operation struct (Query, Insert, DefineEntity, …) implements this
/// trait.  Backends can accept `&dyn Operation` and downcast to the concrete
/// type they support, returning `BackendError::Unsupported` for the rest.
///
/// # Examples
///
/// ```ignore
/// fn handle(op: &dyn Operation) -> Result<(), BackendError> {
///     if let Some(q) = op.as_any().downcast_ref::<Query>() {
///         // handle query
///     } else {
///         Err(BackendError::Unsupported(format!("unsupported: {}", op.kind())))
///     }
/// }
/// ```
pub trait Operation: fmt::Debug + Any {
    /// A human-readable kind label (e.g. `"query"`, `"insert"`).
    fn kind(&self) -> &str;

    /// Downcast support — returns `self` as `&dyn Any`.
    fn as_any(&self) -> &dyn Any;
}

// -- Operation implementations for all operation structs --

impl Operation for DefineEntity {
    fn kind(&self) -> &str { "define_entity" }
    fn as_any(&self) -> &dyn Any { self }
}

impl Operation for AlterEntity {
    fn kind(&self) -> &str { "alter_entity" }
    fn as_any(&self) -> &dyn Any { self }
}

impl Operation for DropEntity {
    fn kind(&self) -> &str { "drop_entity" }
    fn as_any(&self) -> &dyn Any { self }
}

impl Operation for DefineIndex {
    fn kind(&self) -> &str { "define_index" }
    fn as_any(&self) -> &dyn Any { self }
}

impl Operation for DropIndex {
    fn kind(&self) -> &str { "drop_index" }
    fn as_any(&self) -> &dyn Any { self }
}

impl Operation for DefineType {
    fn kind(&self) -> &str { "define_type" }
    fn as_any(&self) -> &dyn Any { self }
}

impl Operation for DropType {
    fn kind(&self) -> &str { "drop_type" }
    fn as_any(&self) -> &dyn Any { self }
}

impl Operation for Insert {
    fn kind(&self) -> &str { "insert" }
    fn as_any(&self) -> &dyn Any { self }
}

impl Operation for InsertSelect {
    fn kind(&self) -> &str { "insert_select" }
    fn as_any(&self) -> &dyn Any { self }
}

impl Operation for Grant {
    fn kind(&self) -> &str { "grant" }
    fn as_any(&self) -> &dyn Any { self }
}

impl Operation for Revoke {
    fn kind(&self) -> &str { "revoke" }
    fn as_any(&self) -> &dyn Any { self }
}

impl Operation for GetObject {
    fn kind(&self) -> &str { "get_object" }
    fn as_any(&self) -> &dyn Any { self }
}

impl Operation for ListObjects {
    fn kind(&self) -> &str { "list_objects" }
    fn as_any(&self) -> &dyn Any { self }
}

impl Operation for ReadFile {
    fn kind(&self) -> &str { "read_file" }
    fn as_any(&self) -> &dyn Any { self }
}

impl Operation for MoveFile {
    fn kind(&self) -> &str { "move_file" }
    fn as_any(&self) -> &dyn Any { self }
}

// ---------------------------------------------------------------------------
// BackendError — shared error type for all backends
// ---------------------------------------------------------------------------

/// Errors that can occur during backend rendering.
///
/// Every DOL backend (SQL, spreadsheet, KV, object-storage, …) uses this
/// shared type so callers can handle errors uniformly.
#[derive(Debug)]
pub enum BackendError {
    /// The backend does not support the requested operation or expression.
    Unsupported(String),
    /// A rendering error that is not an unsupported-feature issue.
    Render(String),
    /// Type or constraint validation failure.
    Validation(String),
    /// An opaque backend-specific error.
    Custom(Box<dyn std::error::Error + Send + Sync>),
}

impl Clone for BackendError {
    fn clone(&self) -> Self {
        match self {
            Self::Unsupported(s) => Self::Unsupported(s.clone()),
            Self::Render(s) => Self::Render(s.clone()),
            Self::Validation(s) => Self::Validation(s.clone()),
            Self::Custom(e) => Self::Render(e.to_string()),
        }
    }
}

impl PartialEq for BackendError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unsupported(a), Self::Unsupported(b)) => a == b,
            (Self::Render(a), Self::Render(b)) => a == b,
            (Self::Validation(a), Self::Validation(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for BackendError {}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(msg) => write!(f, "unsupported: {}", msg),
            Self::Render(msg) => write!(f, "render error: {}", msg),
            Self::Validation(msg) => write!(f, "validation error: {}", msg),
            Self::Custom(e) => write!(f, "backend error: {}", e),
        }
    }
}

impl std::error::Error for BackendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Custom(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

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
