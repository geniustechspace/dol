//! Transaction builder — BEGIN, COMMIT, ROLLBACK, savepoints, blocks.

use dol_ir::transaction::TransactionIR;
use dol_ir::Statement;

/// Builder for transaction control operations.
pub struct TransactionBuilder;

impl TransactionBuilder {
    pub fn begin() -> TransactionIR {
        TransactionIR::Begin
    }

    pub fn commit() -> TransactionIR {
        TransactionIR::Commit
    }

    pub fn rollback() -> TransactionIR {
        TransactionIR::Rollback
    }

    pub fn savepoint(name: &str) -> TransactionIR {
        TransactionIR::Savepoint(name.to_string())
    }

    pub fn release_savepoint(name: &str) -> TransactionIR {
        TransactionIR::ReleaseSavepoint(name.to_string())
    }

    pub fn rollback_to_savepoint(name: &str) -> TransactionIR {
        TransactionIR::RollbackToSavepoint(name.to_string())
    }

    /// Create a transaction block (list of statements to execute atomically).
    pub fn block(stmts: Vec<Statement>) -> TransactionIR {
        TransactionIR::Block(stmts)
    }
}
