//! `Tx` — transaction control. Meta-operation, not DDL/DML/DQL/ACL.

use alloc::vec::Vec;

use crate::operation::Operation;
use crate::target::Symbol;

/// Isolation levels that backends can honour. Backends that do not support
/// a level should diagnose via the capability layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum IsolationLevel {
    /// Minimal isolation; uncommitted changes from other transactions visible.
    ReadUncommitted,
    /// Only committed changes visible, but non-repeatable reads possible.
    ReadCommitted,
    /// Snapshot at statement level; phantom reads possible.
    RepeatableRead,
    /// Full snapshot isolation; no phantoms but write skew possible.
    Snapshot,
    /// Strictest isolation; transactions behave as if run sequentially.
    Serializable,
}

/// Options shared by `TxBegin` and `TxOp::Atomic`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TxOptions {
    /// Requested isolation level, if any.
    pub isolation: Option<IsolationLevel>,
    /// Whether the transaction is read-only.
    pub read_only: bool,
    /// Optional label for debugging / tracing.
    pub label: Option<Symbol>,
}

/// `Begin` payload.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TxBegin {
    /// Transaction options (isolation, read-only, label).
    pub opts: TxOptions,
}

/// Transaction control operation.
///
/// Covers begin/commit/rollback and savepoint management.
///
/// # Examples
///
/// ```
/// use dol_ir::operation::{TxBegin, TxOp, TxOptions, IsolationLevel};
/// use dol_ir::Operation;
///
/// // BEGIN TRANSACTION ISOLATION LEVEL SERIALIZABLE
/// let op: Operation = TxOp::Begin(TxBegin {
///     opts: TxOptions {
///         isolation: Some(IsolationLevel::Serializable),
///         read_only: false,
///         label: None,
///     },
/// })
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::OpKind::Tx);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum TxOp {
    /// Start a new transaction.
    Begin(TxBegin),
    /// Commit the current transaction.
    Commit,
    /// Abort and roll back the current transaction.
    Rollback,
    /// Create a savepoint with the given label.
    Savepoint(Symbol),
    /// Release (drop) a savepoint.
    ReleaseSavepoint(Symbol),
    /// Roll back to a savepoint without ending the transaction.
    RollbackTo(Symbol),
    /// Execute the contained operations atomically.
    Atomic {
        /// Operations to execute as a single atomic unit.
        ops: Vec<Operation>,
        /// Options for the implicit transaction.
        opts: TxOptions,
    },
}
