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

    // ── IR methods ──────────────────────────────────────────────────────

    /// Build a BEGIN as a [`dol_ir::Statement`].
    pub fn begin_ir() -> dol_ir::Statement {
        dol_ir::Statement::Transaction(dol_ir::Transaction::Begin)
    }

    /// Build a COMMIT as a [`dol_ir::Statement`].
    pub fn commit_ir() -> dol_ir::Statement {
        dol_ir::Statement::Transaction(dol_ir::Transaction::Commit)
    }

    /// Build a ROLLBACK as a [`dol_ir::Statement`].
    pub fn rollback_ir() -> dol_ir::Statement {
        dol_ir::Statement::Transaction(dol_ir::Transaction::Rollback)
    }

    /// Build a SAVEPOINT as a [`dol_ir::Statement`].
    pub fn savepoint_ir(name: &str) -> dol_ir::Statement {
        dol_ir::Statement::Transaction(dol_ir::Transaction::Savepoint(name.to_string()))
    }

    /// Build a RELEASE SAVEPOINT as a [`dol_ir::Statement`].
    pub fn release_savepoint_ir(name: &str) -> dol_ir::Statement {
        dol_ir::Statement::Transaction(dol_ir::Transaction::ReleaseSavepoint(name.to_string()))
    }

    /// Build a ROLLBACK TO SAVEPOINT as a [`dol_ir::Statement`].
    pub fn rollback_to_savepoint_ir(name: &str) -> dol_ir::Statement {
        dol_ir::Statement::Transaction(dol_ir::Transaction::RollbackToSavepoint(name.to_string()))
    }
}
