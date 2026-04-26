//! # `dol-check` — static validator
//!
//! Every backend that consumes a [`dol_ir::Program`] is expected to call
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
//!   caller-supplied [`BackendCapabilities`].

#![deny(unsafe_code)]
#![warn(missing_docs)]

use dol_core::diag::Diagnostic;
use dol_ir::{BackendCapabilities, Program, Statement};

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

/// Run [`check_all`] *plus* [`capability_check`] against the given backend
/// capability set.
pub fn check_all_for(program: &Program, caps: BackendCapabilities) -> alloc::vec::Vec<Diagnostic> {
    let mut out = check_all(program);
    capability_check(program, caps, &mut out);
    out
}

/// Verify operand types within the program. Currently a stub that records
/// no findings — the full implementation will inspect each statement's
/// expression tree against `dol-core::DataType::accepts`.
pub fn type_check(_program: &Program, _out: &mut alloc::vec::Vec<Diagnostic>) {
    // Reserved for the full implementation.
}

/// Verify entity / field references resolve. Stub.
pub fn schema_check(_program: &Program, _out: &mut alloc::vec::Vec<Diagnostic>) {
    // Reserved for the full implementation.
}

/// Verify the program's capability requirements are a subset of `caps`.
///
/// Each statement contributes a [`BackendCapabilities`] bitset describing
/// what it depends on; the check fails any statement whose required bits are
/// not present in `caps`.
pub fn capability_check(
    program: &Program,
    caps: BackendCapabilities,
    out: &mut alloc::vec::Vec<Diagnostic>,
) {
    for stmt in core::slice::from_ref(&program.stmt) {
        let required = required_capabilities(stmt);
        let missing = required.difference(caps);
        if !missing.is_empty() {
            out.push(Diagnostic::error(
                dol_core::diag::code::MISSING_CAPABILITY,
                dol_core::span::Span::NONE,
                alloc::format!(
                    "statement requires capabilities not provided by backend: {missing:?}"
                ),
            ));
        }
    }
}

/// Coarse capability requirement for a statement. Refined per-statement
/// inspection is the next step; this baseline returns `empty()` for
/// statements that are universally supported.
fn required_capabilities(stmt: &Statement) -> BackendCapabilities {
    match stmt {
        Statement::Extension(_) => BackendCapabilities::EXTENSIONS,
        _ => BackendCapabilities::empty(),
    }
}

/// Style / best-practice findings. Stub.
pub fn lint(_program: &Program, _out: &mut alloc::vec::Vec<Diagnostic>) {
    // Reserved for future lints, e.g. redundant casts, suspicious predicates.
}

extern crate alloc;
