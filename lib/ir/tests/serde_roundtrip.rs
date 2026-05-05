//! Serde JSON round-trip gate for `dol-ir` public types.
//!
//! Every public IR type must survive a JSON encode → decode cycle. This is
//! a dedicated CI gate (see `.github/workflows/ci.yml`) — failing it is a
//! regression on universal serde.
//!
//! Per-`(Category, TargetKind)` coverage lives in `fixtures.rs`; this file
//! adds compact round-trips for the surrounding shapes (`Program`,
//! `OperationExtension`, transactional ops) so that the gate exercises the
//! full surface and not just the verb payloads.

#![cfg(feature = "serde")]

use dol_expr::{ExprArena, Interner};
use dol_ir::operation::{
    ExtensionId, Insert, InsertSource, OperationExtension, TxBegin, TxOp, TxOptions,
};
use dol_ir::{Locator, Operation, Program, Symbol, Target, TargetKind};
use serde_json::{from_str, to_string};

fn round_trip<T>(v: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let s = to_string(v).expect("serialize");
    from_str(&s).expect("deserialize")
}

#[test]
fn target_round_trip() {
    let t = Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(7)));
    assert_eq!(t, round_trip(&t));
}

#[test]
fn extension_id_round_trip() {
    let id = ExtensionId::new(Symbol::from_hash(42), 3);
    assert_eq!(id, round_trip(&id));
}

#[test]
fn operation_extension_round_trip() {
    let ext = OperationExtension {
        id: ExtensionId::new(Symbol::from_hash(1), 1),
        payload: alloc_vec(b"opaque-bytes"),
    };
    assert_eq!(ext, round_trip(&ext));
}

#[test]
fn tx_op_round_trip() {
    let begin = TxOp::Begin(TxBegin {
        opts: TxOptions::default(),
    });
    assert_eq!(begin, round_trip(&begin));
    assert_eq!(TxOp::Commit, round_trip(&TxOp::Commit));
    assert_eq!(TxOp::Rollback, round_trip(&TxOp::Rollback));
}

#[test]
fn program_with_insert_round_trips() {
    let mut interner = Interner::new();
    let users = Symbol::new(interner.intern("users"));
    let op: Operation = Insert {
        target: Target::new(TargetKind::Relation, Locator::new(users)),
        source: InsertSource::Bindings,
        returning: None,
    }
    .into();
    let program = Program::new(op, ExprArena::new(), interner);
    let decoded: Program = round_trip(&program);
    assert_eq!(program.operations.len(), decoded.operations.len());
    assert_eq!(program.operations[0].kind(), decoded.operations[0].kind());
}

fn alloc_vec(bytes: &[u8]) -> alloc::vec::Vec<u8> {
    bytes.to_vec()
}

extern crate alloc;
