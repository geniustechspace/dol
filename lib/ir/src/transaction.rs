//! Transaction control operations.

use crate::statement::Statement;

/// A transaction control command.
///
/// The `Block` variant wraps a sequence of statements that should be executed
/// as a single atomic unit.  There are no lifetime parameters because
/// [`Statement`] no longer borrows expression trees.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
