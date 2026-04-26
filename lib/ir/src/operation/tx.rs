//! `Tx` — transaction control. Meta-operation, not DDL/DML/DQL/ACL.

use crate::operation::Operation;
use crate::target::Symbol;

/// Isolation levels that backends can honour. Backends that do not support
/// a level should diagnose via the capability layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Snapshot,
    Serializable,
}

/// Options shared by `TxBegin` and `TxOp::Atomic`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TxOptions {
    pub isolation: Option<IsolationLevel>,
    pub read_only: bool,
    pub label: Option<Symbol>,
}

/// `Begin` payload.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TxBegin {
    pub opts: TxOptions,
}

/// Transaction control operation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TxOp {
    Begin(TxBegin),
    Commit,
    Rollback,
    Savepoint(Symbol),
    ReleaseSavepoint(Symbol),
    RollbackTo(Symbol),
    /// Execute the contained operations atomically.
    Atomic {
        ops: Vec<Operation>,
        opts: TxOptions,
    },
}
