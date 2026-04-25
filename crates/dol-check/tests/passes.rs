//! Per-pass smoke tests for `dol-check`.
//!
//! These confirm that each public pass (and the two convenience entry
//! points) exists, is callable on a representative `Program`, and follows
//! the documented append-only contract on the diagnostic list.

use dol_check::{capability_check, check_all, check_all_for, lint, schema_check, type_check};
use dol_diag::Diagnostic;
use dol_ir::{
    BackendCapabilities, Program, Statement, StatementExtension,
    definition::{DefineEntity, FieldDef},
};
use dol_types::DataType;

fn empty_program() -> Program {
    let de = DefineEntity {
        name: "users".into(),
        namespace: None,
        fields: vec![FieldDef::new("id", DataType::Uuid).identity()],
        constraints: vec![],
        if_not_exists: false,
    };
    Program::from_stmt(Statement::from(de))
}

fn extension_program() -> Program {
    Program::from_stmt(Statement::from(StatementExtension {
        id: "test/example".into(),
        payload: vec![1, 2, 3],
    }))
}

#[test]
fn type_check_is_pure_and_append_only() {
    let prog = empty_program();
    let mut diags: Vec<Diagnostic> = Vec::new();
    let seed = Diagnostic::error(
        dol_diag::code::MISSING_CAPABILITY,
        dol_span::Span::NONE,
        "seed".to_string(),
    );
    diags.push(seed.clone());
    type_check(&prog, &mut diags);
    // Stub today, but must never *remove* prior diagnostics.
    assert!(!diags.is_empty());
    assert_eq!(diags[0].message, seed.message);
}

#[test]
fn schema_check_is_pure_and_append_only() {
    let prog = empty_program();
    let mut diags: Vec<Diagnostic> = Vec::new();
    schema_check(&prog, &mut diags);
    assert!(diags.is_empty(), "stub must be a no-op on its own");
}

#[test]
fn lint_is_pure_and_append_only() {
    let prog = empty_program();
    let mut diags: Vec<Diagnostic> = Vec::new();
    lint(&prog, &mut diags);
    assert!(diags.is_empty(), "stub must be a no-op on its own");
}

#[test]
fn capability_check_passes_when_capabilities_are_present() {
    let prog = extension_program();
    let mut diags: Vec<Diagnostic> = Vec::new();
    capability_check(&prog, BackendCapabilities::ALL, &mut diags);
    assert!(diags.is_empty(), "ALL must accept any capability");
}

#[test]
fn capability_check_flags_missing_capabilities() {
    let prog = extension_program();
    let mut diags: Vec<Diagnostic> = Vec::new();
    capability_check(&prog, BackendCapabilities::empty(), &mut diags);
    assert_eq!(diags.len(), 1, "extension requires EXTENSIONS capability");
    assert_eq!(diags[0].code, dol_diag::code::MISSING_CAPABILITY);
}

#[test]
fn capability_check_no_op_on_universally_supported_statements() {
    let prog = empty_program();
    let mut diags: Vec<Diagnostic> = Vec::new();
    capability_check(&prog, BackendCapabilities::empty(), &mut diags);
    assert!(diags.is_empty(), "DDL needs no extra capabilities");
}

#[test]
fn check_all_runs_only_static_passes() {
    // `check_all` deliberately does NOT run `capability_check`; an extension
    // statement with an empty backend capability set must therefore produce
    // no diagnostics here.
    let prog = extension_program();
    let diags = check_all(&prog);
    assert!(
        diags.is_empty(),
        "check_all must not invoke capability_check (got {diags:?})"
    );
}

#[test]
fn check_all_for_runs_capability_check() {
    let prog = extension_program();
    let diags = check_all_for(&prog, BackendCapabilities::empty());
    assert_eq!(
        diags.len(),
        1,
        "check_all_for must run capability_check (got {diags:?})"
    );
    assert_eq!(diags[0].code, dol_diag::code::MISSING_CAPABILITY);
}

#[test]
fn check_all_for_clean_when_capabilities_satisfy() {
    let prog = extension_program();
    let diags = check_all_for(&prog, BackendCapabilities::ALL);
    assert!(diags.is_empty());
}
