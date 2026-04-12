//! Transaction IR — canonical representation of transaction operations.

/// Transaction operations.
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionIR {
    Begin,
    Commit,
    Rollback,
    Savepoint(String),
    ReleaseSavepoint(String),
    RollbackToSavepoint(String),
    /// Block form: `transaction { stmt1; stmt2; }` — future.
    Block(Vec<super::Statement>),
}
