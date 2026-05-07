//! # `dol-check` — static validator
//!
//! Every backend that consumes a [`dol_ir::program::Program`] is expected to call
//! these checks first. They are pure functions that produce a list of
//! [`dol_core::diag::Diagnostic`]s; an empty list means the program is well-formed.
//!
//! The checker is split into independent passes:
//!
//! - [`type_check`] — operand types are consistent.
//! - [`schema_check`] — entity / field references exist and are valid.
//! - [`capability_check`] — the program does not use features the chosen
//!   backend lacks.
//! - [`lint`] — style / best-practice findings.
//!
//! All passes are append-only on the diagnostic list: callers may run only
//! the passes they need.
//!
//! Two convenience entry points cover the common cases:
//!
//! - [`check_all`] runs the three backend-agnostic passes
//!   ([`type_check`], [`schema_check`], [`lint`]).
//! - [`check_all_for`] additionally runs [`capability_check`] against a
//!   caller-supplied [`CapabilitySet`].
//!
//! Diagnostics from [`capability_check`] use the structured
//! [`CapabilityCheck`] key so message text is uniform across backends.

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

pub mod sarif;

use dol_core::diag::Diagnostic;
use dol_ir::capabilities::{CapabilityCheck, CapabilitySet, CapabilityTag};
use dol_ir::operation::{Category, OpKind, Operation};
use dol_ir::program::Program;

/// Run the backend-agnostic passes ([`type_check`], [`schema_check`],
/// [`lint`]) in order.
///
/// Use [`check_all_for`] when you also need [`capability_check`] against a
/// concrete backend.
pub fn check_all(program: &Program) -> alloc::vec::Vec<Diagnostic> {
    let mut out = alloc::vec::Vec::new();
    type_check(program, &mut out);
    schema_check(program, &mut out);
    lint(program, &mut out);
    out
}

/// Run [`check_all`] *plus* [`capability_check`] against the given
/// [`CapabilitySet`].
pub fn check_all_for(program: &Program, provided: &CapabilitySet) -> alloc::vec::Vec<Diagnostic> {
    let mut out = check_all(program);
    capability_check(program, provided, &mut out);
    out
}

/// Verify operand types within the program. Currently a stub that records
/// no findings — the full implementation will inspect each operation's
/// expression tree against `dol-core::DataType::accepts`.
pub fn type_check(_program: &Program, _out: &mut alloc::vec::Vec<Diagnostic>) {
    // Reserved for the full implementation.
}

/// Verify entity / field references resolve. Stub.
pub fn schema_check(_program: &Program, _out: &mut alloc::vec::Vec<Diagnostic>) {
    // Reserved for the full implementation.
}

/// Verify the program's capability requirements are a subset of `provided`.
///
/// Each [`Operation`] contributes a [`CapabilitySet`] describing what it
/// depends on; the check fails any tag that is not present in `provided`
/// and emits one diagnostic per missing tag with a structured
/// [`CapabilityCheck`] key.
pub fn capability_check(
    program: &Program,
    provided: &CapabilitySet,
    out: &mut alloc::vec::Vec<Diagnostic>,
) {
    for op in &program.operations {
        check_op_recursive(op, provided, out);
    }
}

/// Walk into [`TxOp::Atomic`] payloads so missing capabilities inside an
/// atomic block are diagnosed alongside the outer `Tx` operation. Mirrors
/// the recursive contract documented on
/// [`Operation::all_targets`](dol_ir::operation::Operation::all_targets).
fn check_op_recursive(
    op: &Operation,
    provided: &CapabilitySet,
    out: &mut alloc::vec::Vec<Diagnostic>,
) {
    let required = op.required_capabilities();
    for missing in required.missing_from(provided) {
        let check = capability_check_for(op, &missing);
        let msg = render_capability_message(&check, &missing);
        out.push(Diagnostic::error(
            dol_core::diag::code::MISSING_CAPABILITY,
            dol_core::span::Span::NONE,
            msg,
        ));
    }
    if let Operation::Tx(tx) = op {
        if let dol_ir::operation::TxOp::Atomic { ops, .. } = tx.as_ref() {
            for inner in ops {
                check_op_recursive(inner, provided, out);
            }
        }
    }
}

/// Build a [`CapabilityCheck`] key for the given operation / missing tag.
fn capability_check_for(op: &Operation, _missing: &CapabilityTag) -> CapabilityCheck {
    let mut c = CapabilityCheck::new(op.kind());
    if let Some(t) = op.primary_target() {
        c = c.with_target(t.kind.clone());
    }
    match op {
        Operation::Schema(s) => {
            c = c.with_verb(s.verb);
        }
        Operation::Field(s) => {
            c = c.with_verb(s.verb);
        }
        Operation::Index(s) => {
            c = c.with_verb(s.verb);
        }
        Operation::Lookup(s) => {
            c = c.with_verb(s.verb);
        }
        Operation::Policy(s) => {
            c = c.with_verb(s.verb);
        }
        Operation::Mask(s) => {
            c = c.with_verb(s.verb);
        }
        Operation::Quota(s) => {
            c = c.with_verb(s.verb);
        }
        Operation::Audit(s) => {
            c = c.with_verb(s.verb);
        }
        _ => {}
    }
    c
}

/// Render a uniform, backend-agnostic capability diagnostic message.
fn render_capability_message(
    check: &CapabilityCheck,
    missing: &CapabilityTag,
) -> alloc::string::String {
    use core::fmt::Write;
    let mut s = alloc::string::String::new();
    let _ = write!(s, "operation {:?}", check.op);
    if let Some(t) = &check.target {
        let _ = write!(s, " on TargetKind::{t:?}");
    }
    if let Some(v) = check.verb {
        let _ = write!(s, " ({v:?})");
    }
    let _ = write!(
        s,
        " requires capability `{missing}` not provided by backend"
    );
    s
}

/// Style / best-practice findings. Stub.
pub fn lint(_program: &Program, _out: &mut alloc::vec::Vec<Diagnostic>) {
    // Reserved for future lints, e.g. redundant casts, suspicious predicates.
}

// ── Compatibility entry point ─────────────────────────────────────────────

/// Legacy bitset-based capability check. Forwards through the new
/// [`capability_check`] by translating the bitset's set bits into
/// [`CapabilityTag`]s.
///
/// Provided so existing callers can continue to pass [`dol_ir::capabilities::BackendCapabilities`].
#[deprecated(
    since = "0.2.0",
    note = "use `capability_check` with a `CapabilitySet`"
)]
pub fn capability_check_bits(
    program: &Program,
    caps: dol_ir::capabilities::BackendCapabilities,
    out: &mut alloc::vec::Vec<Diagnostic>,
) {
    let provided = bits_to_set(caps);
    capability_check(program, &provided, out);
}

fn bits_to_set(caps: dol_ir::capabilities::BackendCapabilities) -> CapabilitySet {
    use dol_ir::capabilities::BackendCapabilities as B;
    let mut s = CapabilitySet::new();
    let table: &[(B, CapabilityTag)] = &[
        (B::WINDOW_FUNCTIONS, CapabilityTag::WINDOW_FUNCTIONS),
        (B::RECURSIVE_CTE, CapabilityTag::RECURSIVE_CTE),
        (B::JSON_ARROWS, CapabilityTag::JSON_ARROWS),
        (B::VECTOR_INDEX, CapabilityTag::VECTOR_INDEX),
        (B::GEOSPATIAL, CapabilityTag::GEOSPATIAL),
        (B::MERGE, CapabilityTag::MERGE),
        (B::ROW_LOCKING, CapabilityTag::ROW_LOCKING),
        (B::LOCK_SKIP_NOWAIT, CapabilityTag::LOCK_SKIP_NOWAIT),
        (B::STREAMING_WINDOWS, CapabilityTag::STREAMING_WINDOWS),
        (B::TIME_SERIES, CapabilityTag::TIME_SERIES),
        (B::PIPELINES, CapabilityTag::PIPELINES),
        (B::OBJECT_STORE, CapabilityTag::OBJECT_STORE),
        (B::FILE_IO, CapabilityTag::FILE_IO),
        (B::POLICIES, CapabilityTag::POLICIES),
        (B::EXTENSIONS, CapabilityTag::EXTENSIONS),
        (B::REPLACE_OP, CapabilityTag::REPLACE_OP),
        (B::PROBE_OP, CapabilityTag::PROBE_OP),
        (B::DESCRIBE_OP, CapabilityTag::DESCRIBE_OP),
        (B::APPEND_OP, CapabilityTag::APPEND_OP),
        (B::MULTI_STATEMENT_TX, CapabilityTag::MULTI_STATEMENT_TX),
        (B::MASK_POLICIES, CapabilityTag::MASK_POLICIES),
        (B::QUOTAS, CapabilityTag::QUOTAS),
        (B::AUDIT, CapabilityTag::AUDIT),
        (B::OPAQUE_SCHEMA, CapabilityTag::OPAQUE_SCHEMA),
        (B::BLOB_TARGETS, CapabilityTag::BLOB_TARGETS),
        (B::FILE_TREE_TARGETS, CapabilityTag::FILE_TREE_TARGETS),
        (B::STREAM_TARGETS, CapabilityTag::STREAM_TARGETS),
        (B::API_TARGETS, CapabilityTag::API_TARGETS),
        (B::RAW_PASSTHROUGH, CapabilityTag::RAW_PASSTHROUGH),
    ];
    for (bit, tag) in table {
        if caps.contains(*bit) {
            s.insert(tag.clone());
        }
    }
    s
}

/// Re-export so callers can name [`OpKind`] without depending on `dol-ir` directly.
pub use dol_ir::operation::OpKind as ReexportedOpKind;
const _: fn() = || {
    let _ = OpKind::Append;
};

#[cfg(test)]
mod tests {
    use super::*;
    use dol_ir::operation::{Append, InsertSource};
    use dol_ir::operation::Operation;
    use dol_ir::program::Program;
    use dol_ir::target::{Locator, Symbol, Target, TargetKind};

    #[test]
    fn append_on_stream_topic_diagnoses_missing_capabilities() {
        let op: Operation = Append {
            target: Target::new(TargetKind::StreamTopic, Locator::new(Symbol::default())),
            source: InsertSource::Bindings,
            partition_key: None,
        }
        .into();
        let prog = Program::from_operation(op);
        let provided = CapabilitySet::new();
        let mut out = alloc::vec::Vec::new();
        capability_check(&prog, &provided, &mut out);
        assert!(out.iter().any(|d| d.message.contains("Append")));
        assert!(out.iter().any(|d| d.message.contains("StreamTopic")));
    }
}
