//! Transaction builder — BEGIN, COMMIT, ROLLBACK, savepoints.

/// Builder for transaction control operations.
pub struct TransactionBuilder;

impl TransactionBuilder {
    /// Build a BEGIN as a [`dol_ir::Statement`].
    pub fn begin() -> dol_ir::Statement {
        dol_ir::Statement::Transaction(dol_ir::Transaction::Begin)
    }

    /// Build a COMMIT as a [`dol_ir::Statement`].
    pub fn commit() -> dol_ir::Statement {
        dol_ir::Statement::Transaction(dol_ir::Transaction::Commit)
    }

    /// Build a ROLLBACK as a [`dol_ir::Statement`].
    pub fn rollback() -> dol_ir::Statement {
        dol_ir::Statement::Transaction(dol_ir::Transaction::Rollback)
    }

    /// Build a SAVEPOINT as a [`dol_ir::Statement`].
    pub fn savepoint(name: &str) -> dol_ir::Statement {
        dol_ir::Statement::Transaction(dol_ir::Transaction::Savepoint(name.to_string()))
    }

    /// Build a RELEASE SAVEPOINT as a [`dol_ir::Statement`].
    pub fn release_savepoint(name: &str) -> dol_ir::Statement {
        dol_ir::Statement::Transaction(dol_ir::Transaction::ReleaseSavepoint(name.to_string()))
    }

    /// Build a ROLLBACK TO SAVEPOINT as a [`dol_ir::Statement`].
    pub fn rollback_to_savepoint(name: &str) -> dol_ir::Statement {
        dol_ir::Statement::Transaction(dol_ir::Transaction::RollbackToSavepoint(name.to_string()))
    }
}
