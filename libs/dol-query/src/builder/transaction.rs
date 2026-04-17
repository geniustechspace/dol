//! Transaction builder — BEGIN, COMMIT, ROLLBACK, savepoints, blocks.

use dol_core::ir::Statement;
use dol_core::ir::transaction::TransactionIR;

/// Builder for transaction control operations.
pub struct TransactionBuilder;

impl TransactionBuilder {
    pub fn begin<'a>() -> TransactionIR<'a> {
        TransactionIR::Begin
    }

    pub fn commit<'a>() -> TransactionIR<'a> {
        TransactionIR::Commit
    }

    pub fn rollback<'a>() -> TransactionIR<'a> {
        TransactionIR::Rollback
    }

    pub fn savepoint<'a>(name: &str) -> TransactionIR<'a> {
        TransactionIR::Savepoint(name.to_string())
    }

    pub fn release_savepoint<'a>(name: &str) -> TransactionIR<'a> {
        TransactionIR::ReleaseSavepoint(name.to_string())
    }

    pub fn rollback_to_savepoint<'a>(name: &str) -> TransactionIR<'a> {
        TransactionIR::RollbackToSavepoint(name.to_string())
    }

    /// Create a transaction block (list of statements to execute atomically).
    pub fn block<'a>(stmts: Vec<Statement<'a>>) -> TransactionIR<'a> {
        TransactionIR::Block(stmts)
    }
}
