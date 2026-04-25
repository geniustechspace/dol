//! Transaction control operations.

use crate::statement::Statement;

/// A transaction control command.
///
/// The `Block` variant wraps a sequence of statements that should be executed
/// as a single atomic unit.  There are no lifetime parameters because
/// [`Statement`] no longer borrows expression trees.
///
/// Note: `Transaction` does not derive `serde::{Serialize, Deserialize}` because
/// the `Block` variant contains [`Statement`], which includes DML variants that
/// do not yet implement those traits.  Serde support will be added once the
/// arena types in `dol-expr` gain Serialize/Deserialize.
#[derive(Debug, Clone, PartialEq)]
pub enum Transaction {
    Begin,
    Commit,
    Rollback,
    Savepoint(String),
    ReleaseSavepoint(String),
    RollbackToSavepoint(String),
    /// Execute the contained statements as an atomic block.
    Block(Vec<Statement>),
}
