//! Transaction builder — BEGIN, COMMIT, ROLLBACK, savepoints, blocks.

use dol_core::op::Statement;
use dol_core::op::transaction::Transaction;

/// Builder for transaction control operations.
pub struct TransactionBuilder;

impl TransactionBuilder {
    pub fn begin<'a>() -> Transaction<'a> {
        Transaction::Begin
    }

    pub fn commit<'a>() -> Transaction<'a> {
        Transaction::Commit
    }

    pub fn rollback<'a>() -> Transaction<'a> {
        Transaction::Rollback
    }

    pub fn savepoint<'a>(name: &str) -> Transaction<'a> {
        Transaction::Savepoint(name.to_string())
    }

    pub fn release_savepoint<'a>(name: &str) -> Transaction<'a> {
        Transaction::ReleaseSavepoint(name.to_string())
    }

    pub fn rollback_to_savepoint<'a>(name: &str) -> Transaction<'a> {
        Transaction::RollbackToSavepoint(name.to_string())
    }

    /// Create a transaction block (list of statements to execute atomically).
    pub fn block<'a>(stmts: Vec<Statement<'a>>) -> Transaction<'a> {
        Transaction::Block(stmts)
    }
}
