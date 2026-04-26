//! Access-control + transaction helpers — emit v2 [`Operation`]s for
//! `Grant`, `Revoke`, `Policy`, `Tx`.

use dol_expr::{ExprArena, Interner};
use dol_ir::operation::{
    GrantV2, IsolationLevel, PolicyOp, PolicyScope, RevokeV2, StructuralVerb, TxBegin, TxOp,
    TxOptions,
};
use dol_ir::{Operation, Privilege, Program, Symbol, TargetKind};
use smallvec::SmallVec;

use crate::target::{intern_symbol, target_from_parts};

/// Emit `Operation::Grant`.
pub fn grant(
    privileges: Vec<Privilege>,
    target_name: &str,
    target_namespace: Option<&str>,
    target_kind: TargetKind,
    roles: Vec<&str>,
    with_grant_option: bool,
) -> Program {
    let mut interner = Interner::new();
    let target = target_from_parts(&mut interner, target_kind, target_name, target_namespace);
    let role_syms: SmallVec<[Symbol; 1]> = roles
        .iter()
        .map(|r| intern_symbol(&mut interner, r))
        .collect();
    let priv_vec: SmallVec<[Privilege; 2]> = privileges.into_iter().collect();
    let op: Operation = GrantV2 {
        privileges: priv_vec,
        target,
        roles: role_syms,
        with_grant_option,
    }
    .into();
    Program::new(op, ExprArena::new(), interner)
}

/// Emit `Operation::Revoke`.
pub fn revoke(
    privileges: Vec<Privilege>,
    target_name: &str,
    target_namespace: Option<&str>,
    target_kind: TargetKind,
    roles: Vec<&str>,
) -> Program {
    let mut interner = Interner::new();
    let target = target_from_parts(&mut interner, target_kind, target_name, target_namespace);
    let role_syms: SmallVec<[Symbol; 1]> = roles
        .iter()
        .map(|r| intern_symbol(&mut interner, r))
        .collect();
    let priv_vec: SmallVec<[Privilege; 2]> = privileges.into_iter().collect();
    let op: Operation = RevokeV2 {
        privileges: priv_vec,
        target,
        roles: role_syms,
        cascade: false,
    }
    .into();
    Program::new(op, ExprArena::new(), interner)
}

/// Emit `Operation::Policy { verb: Create, … }`.
pub fn define_policy(
    name: &str,
    table: &str,
    namespace: Option<&str>,
    scope: PolicyScope,
) -> Program {
    let mut interner = Interner::new();
    let target = target_from_parts(&mut interner, TargetKind::Relation, table, namespace);
    let name_sym = intern_symbol(&mut interner, name);
    let op: Operation = PolicyOp {
        verb: StructuralVerb::Create,
        target,
        name: name_sym,
        scope,
        using_expr: None,
        check_expr: None,
    }
    .into();
    Program::new(op, ExprArena::new(), interner)
}

/// Emit `Operation::Tx(TxOp::Begin)`.
pub fn tx_begin(isolation: Option<IsolationLevel>, read_only: bool) -> Program {
    let op: Operation = Operation::Tx(Box::new(TxOp::Begin(TxBegin {
        opts: TxOptions {
            isolation,
            read_only,
            label: None,
        },
    })));
    Program::from_operation(op)
}

/// Emit `Operation::Tx(TxOp::Commit)`.
pub fn tx_commit() -> Program {
    Program::from_operation(Operation::Tx(Box::new(TxOp::Commit)))
}

/// Emit `Operation::Tx(TxOp::Rollback)` or `RollbackTo(label)` when `to` is
/// supplied.
pub fn tx_rollback(to: Option<&str>) -> Program {
    let mut interner = Interner::new();
    let op: Operation = match to {
        Some(label) => {
            let sym = intern_symbol(&mut interner, label);
            Operation::Tx(Box::new(TxOp::RollbackTo(sym)))
        }
        None => Operation::Tx(Box::new(TxOp::Rollback)),
    };
    Program::new(op, ExprArena::new(), interner)
}

/// Emit `Operation::Tx(TxOp::Atomic { ops })` wrapping a sequence of
/// inner operations.
pub fn tx_atomic(ops: Vec<Operation>) -> Program {
    let op: Operation = Operation::Tx(Box::new(TxOp::Atomic {
        ops,
        opts: TxOptions::default(),
    }));
    Program::from_operation(op)
}

// Public re-exports for callers building transactional blocks.
pub use dol_ir::operation::{IsolationLevel as TxIsolationLevel, PolicyScope as PolicyScopeV2};
