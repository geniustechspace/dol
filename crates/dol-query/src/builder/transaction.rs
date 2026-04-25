//! Transaction builder — BEGIN, COMMIT, ROLLBACK, savepoints.

/// Builder for transaction control operations.
pub struct TransactionBuilder;

impl TransactionBuilder {
    /// Build a BEGIN as a [`dol_ir::Statement`].
    pub fn begin() -> dol_ir::Statement {
        dol_ir::Statement::Transaction(Box::new(dol_ir::Transaction::Begin))
    }

    /// Build a COMMIT as a [`dol_ir::Statement`].
    pub fn commit() -> dol_ir::Statement {
        dol_ir::Statement::Transaction(Box::new(dol_ir::Transaction::Commit))
    }

    /// Build a ROLLBACK as a [`dol_ir::Statement`].
    pub fn rollback() -> dol_ir::Statement {
        dol_ir::Statement::Transaction(Box::new(dol_ir::Transaction::Rollback))
    }

    /// Build a SAVEPOINT as a [`dol_ir::Statement`].
    pub fn savepoint(name: &str) -> dol_ir::Statement {
        dol_ir::Statement::Transaction(Box::new(dol_ir::Transaction::Savepoint(name.to_string())))
    }

    /// Build a RELEASE SAVEPOINT as a [`dol_ir::Statement`].
    pub fn release_savepoint(name: &str) -> dol_ir::Statement {
        dol_ir::Statement::Transaction(Box::new(dol_ir::Transaction::ReleaseSavepoint(
            name.to_string(),
        )))
    }

    /// Build a ROLLBACK TO SAVEPOINT as a [`dol_ir::Statement`].
    pub fn rollback_to_savepoint(name: &str) -> dol_ir::Statement {
        dol_ir::Statement::Transaction(Box::new(dol_ir::Transaction::RollbackToSavepoint(
            name.to_string(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::TransactionBuilder;

    #[test]
    fn begin_builds_begin_transaction_statement() {
        assert_eq!(
            TransactionBuilder::begin(),
            dol_ir::Statement::Transaction(Box::new(dol_ir::Transaction::Begin))
        );
    }

    #[test]
    fn commit_builds_commit_transaction_statement() {
        assert_eq!(
            TransactionBuilder::commit(),
            dol_ir::Statement::Transaction(Box::new(dol_ir::Transaction::Commit))
        );
    }

    #[test]
    fn rollback_builds_rollback_transaction_statement() {
        assert_eq!(
            TransactionBuilder::rollback(),
            dol_ir::Statement::Transaction(Box::new(dol_ir::Transaction::Rollback))
        );
    }

    #[test]
    fn savepoint_builds_savepoint_transaction_statement_with_name() {
        let name = "sp1";
        assert_eq!(
            TransactionBuilder::savepoint(name),
            dol_ir::Statement::Transaction(Box::new(dol_ir::Transaction::Savepoint(
                name.to_string()
            )))
        );
    }

    #[test]
    fn release_savepoint_builds_release_savepoint_transaction_statement_with_name() {
        let name = "sp1";
        assert_eq!(
            TransactionBuilder::release_savepoint(name),
            dol_ir::Statement::Transaction(Box::new(dol_ir::Transaction::ReleaseSavepoint(
                name.to_string()
            )))
        );
    }

    #[test]
    fn rollback_to_savepoint_builds_rollback_to_savepoint_transaction_statement_with_name() {
        let name = "sp1";
        assert_eq!(
            TransactionBuilder::rollback_to_savepoint(name),
            dol_ir::Statement::Transaction(Box::new(dol_ir::Transaction::RollbackToSavepoint(
                name.to_string()
            )))
        );
    }
}
