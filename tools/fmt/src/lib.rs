//! # `dol-fmt` — canonical pretty-printer
//!
//! Renders an IR [`dol_ir::Program`] into a stable, human-readable text form.
//! The output is **not** a parser surface; it exists for debugging,
//! `insta`-style golden tests, and round-trip checks against `dol-wire`.
//!
//! The text form is loosely S-expression-flavoured. With the surrounding
//! [`dol_expr::Interner`] supplying name strings the renderer produces
//! shapes such as:
//!
//! ```text
//! (program
//!   (tx :begin)
//!   (insert :target relation/users)
//!   (tx :commit)
//! )
//! ```
//!
//! The exact shape is governed by snapshot tests; downstream tooling should
//! treat any change as a breaking change.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )
)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::string::String;
use core::fmt::Write;

use dol_expr::Interner;
use dol_ir::operation::TxOp;
use dol_ir::target::TargetKind;
use dol_ir::{OpKind, Operation, Program, Target};

/// Pretty-print a [`Program`] to a `String`.
pub fn print(program: &Program) -> String {
    let mut out = String::new();
    let _ = write_program(&mut out, program);
    out
}

fn write_program(w: &mut String, p: &Program) -> core::fmt::Result {
    writeln!(w, "(program")?;
    for op in &p.operations {
        w.push_str("  ");
        write_operation(w, op, &p.interner)?;
        w.push('\n');
    }
    write!(w, ")")
}

fn write_operation(w: &mut String, op: &Operation, interner: &Interner) -> core::fmt::Result {
    let verb = verb_token(op.kind());
    if let Operation::Tx(tx) = op {
        return write!(w, "({verb} :{})", tx_subverb(tx));
    }
    if let Some(t) = op.primary_target() {
        write!(w, "({verb} :target ")?;
        write_target(w, t, interner)?;
        write!(w, ")")
    } else {
        write!(w, "({verb})")
    }
}

fn write_target(w: &mut String, t: &Target, interner: &Interner) -> core::fmt::Result {
    let kind = target_kind_token(&t.kind);
    let name = resolve(interner, t.locator.name.id());
    if let Some(ns) = t.locator.namespace {
        let ns_str = resolve(interner, ns.id());
        write!(w, "{kind}/{ns_str}.{name}")
    } else {
        write!(w, "{kind}/{name}")
    }
}

/// Resolve `id` against `interner`, falling back to a stable `#<id>`
/// placeholder when the symbol came from a different interner (e.g. test
/// fixtures that build a [`Symbol`](dol_ir::Symbol) from a literal).
///
/// With content-addressed [`StrId`](dol_expr::ids::StrId)s a known id can
/// land anywhere in `u32` space, so we ask the interner directly via
/// [`Interner::get_opt`] rather than treating the id as a sequential
/// index.
fn resolve(interner: &Interner, id: dol_expr::ids::StrId) -> alloc::borrow::Cow<'_, str> {
    match interner.get_opt(id) {
        Some(s) => alloc::borrow::Cow::Borrowed(s),
        None => {
            let mut s = String::new();
            let _ = write!(s, "#{id}");
            alloc::borrow::Cow::Owned(s)
        }
    }
}

fn verb_token(k: OpKind) -> &'static str {
    match k {
        OpKind::Schema => "schema",
        OpKind::Field => "field",
        OpKind::Index => "index",
        OpKind::Lookup => "lookup",
        OpKind::Policy => "policy",
        OpKind::Mask => "mask",
        OpKind::Quota => "quota",
        OpKind::Audit => "audit",
        OpKind::Insert => "insert",
        OpKind::Update => "update",
        OpKind::Replace => "replace",
        OpKind::Delete => "delete",
        OpKind::Upsert => "upsert",
        OpKind::Append => "append",
        OpKind::Query => "query",
        OpKind::Probe => "probe",
        OpKind::Describe => "describe",
        OpKind::Grant => "grant",
        OpKind::Revoke => "revoke",
        OpKind::Tx => "tx",
        OpKind::Extension => "extension",
        // `OpKind::Raw` is feature-gated on `dol-ir/raw`; this catch-all
        // covers it without leaking the feature flag into `dol-fmt`'s
        // public surface.
        #[allow(unreachable_patterns)]
        _ => "raw",
    }
}

fn target_kind_token(k: &TargetKind) -> &'static str {
    match k {
        TargetKind::Relation => "relation",
        TargetKind::Document => "document",
        TargetKind::KeyValue => "kv",
        TargetKind::Blob => "blob",
        TargetKind::FileTree => "filetree",
        TargetKind::StreamTopic => "stream",
        TargetKind::ApiResource => "api",
        TargetKind::Virtual => "virtual",
        TargetKind::Custom(_) => "custom",
    }
}

fn tx_subverb(tx: &TxOp) -> &'static str {
    match tx {
        TxOp::Begin(_) => "begin",
        TxOp::Commit => "commit",
        TxOp::Rollback => "rollback",
        TxOp::Savepoint(_) => "savepoint",
        TxOp::ReleaseSavepoint(_) => "release",
        TxOp::RollbackTo(_) => "rollback-to",
        TxOp::Atomic { .. } => "atomic",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dol_expr::{ExprArena, Interner};
    use dol_ir::operation::{Insert, InsertSource, TxBegin, TxOp, TxOptions};
    use dol_ir::{Locator, Program, Symbol, Target, TargetKind};

    #[test]
    fn renders_lowercase_verb_and_locator_name() {
        let mut interner = Interner::new();
        let users = Symbol::new(interner.intern("users"));
        let op: dol_ir::Operation = Insert {
            target: Target::new(TargetKind::Relation, Locator::new(users)),
            source: InsertSource::Bindings,
            returning: None,
        }
        .into();
        let p = Program::new(op, ExprArena::new(), interner);
        let s = print(&p);
        assert!(
            s.contains("(insert :target relation/users)"),
            "unexpected render: {s}"
        );
    }

    #[test]
    fn renders_tx_subverb() {
        let interner = Interner::new();
        let op: dol_ir::Operation =
            dol_ir::Operation::Tx(alloc::boxed::Box::new(TxOp::Begin(TxBegin {
                opts: TxOptions::default(),
            })));
        let p = Program::new(op, ExprArena::new(), interner);
        let s = print(&p);
        assert!(s.contains("(tx :begin)"), "unexpected render: {s}");
    }
}
