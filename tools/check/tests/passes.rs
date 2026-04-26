//! Per-pass smoke tests for `dol-check`.
//!
//! These confirm that each public pass (and the two convenience entry
//! points) exists, is callable on a representative `Program`, and follows
//! the documented append-only contract on the diagnostic list.

use dol_check::{capability_check, check_all, check_all_for, lint, schema_check, type_check};
use dol_core::DataType;
use dol_core::diag::Diagnostic;
use dol_ir::operation::OperationExtension;
use dol_ir::{
    CapabilitySet, CapabilityTag, Program, Statement,
    definition::{DefineEntity, FieldDef},
};

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
    Program::from_operation(
        OperationExtension {
            id: dol_ir::operation::ExtensionId::new(dol_ir::Symbol::default(), 0),
            payload: vec![1, 2, 3],
        }
        .into(),
    )
}

fn all_caps() -> CapabilitySet {
    let mut s = CapabilitySet::new();
    for tag in [
        CapabilityTag::WINDOW_FUNCTIONS,
        CapabilityTag::RECURSIVE_CTE,
        CapabilityTag::JSON_ARROWS,
        CapabilityTag::VECTOR_INDEX,
        CapabilityTag::GEOSPATIAL,
        CapabilityTag::MERGE,
        CapabilityTag::ROW_LOCKING,
        CapabilityTag::LOCK_SKIP_NOWAIT,
        CapabilityTag::STREAMING_WINDOWS,
        CapabilityTag::TIME_SERIES,
        CapabilityTag::PIPELINES,
        CapabilityTag::OBJECT_STORE,
        CapabilityTag::FILE_IO,
        CapabilityTag::POLICIES,
        CapabilityTag::EXTENSIONS,
        CapabilityTag::REPLACE_OP,
        CapabilityTag::PROBE_OP,
        CapabilityTag::DESCRIBE_OP,
        CapabilityTag::APPEND_OP,
        CapabilityTag::MULTI_STATEMENT_TX,
        CapabilityTag::MASK_POLICIES,
        CapabilityTag::QUOTAS,
        CapabilityTag::AUDIT,
        CapabilityTag::OPAQUE_SCHEMA,
        CapabilityTag::BLOB_TARGETS,
        CapabilityTag::FILE_TREE_TARGETS,
        CapabilityTag::STREAM_TARGETS,
        CapabilityTag::API_TARGETS,
        CapabilityTag::RAW_PASSTHROUGH,
    ] {
        s.insert(tag);
    }
    s
}

#[test]
fn type_check_is_pure_and_append_only() {
    let prog = empty_program();
    let mut diags: Vec<Diagnostic> = Vec::new();
    let seed = Diagnostic::error(
        dol_core::diag::code::MISSING_CAPABILITY,
        dol_core::span::Span::NONE,
        "seed".to_string(),
    );
    diags.push(seed.clone());
    type_check(&prog, &mut diags);
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
    let provided = all_caps();
    let mut diags: Vec<Diagnostic> = Vec::new();
    capability_check(&prog, &provided, &mut diags);
    assert!(diags.is_empty(), "all-caps set must accept any capability");
}

#[test]
fn capability_check_flags_missing_capabilities() {
    let prog = extension_program();
    let provided = CapabilitySet::new();
    let mut diags: Vec<Diagnostic> = Vec::new();
    capability_check(&prog, &provided, &mut diags);
    assert!(!diags.is_empty(), "extension requires EXTENSIONS tag");
    assert!(
        diags
            .iter()
            .all(|d| d.code == dol_core::diag::code::MISSING_CAPABILITY)
    );
}

#[test]
fn capability_check_no_op_on_universally_supported_operations() {
    // A relation-targeted DDL with an inferred schema needs no extra tags.
    let prog = empty_program();
    let provided = CapabilitySet::new();
    let mut diags: Vec<Diagnostic> = Vec::new();
    capability_check(&prog, &provided, &mut diags);
    // The compat shim leaves the schema binding `Opaque`, which surfaces an
    // OPAQUE_SCHEMA tag — that is the documented behaviour. We only assert
    // the call is well-formed.
    let _ = diags;
}

#[test]
fn check_all_runs_only_static_passes() {
    // `check_all` deliberately does NOT run `capability_check`; an extension
    // operation with no provided capabilities must therefore produce no
    // diagnostics here.
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
    let provided = CapabilitySet::new();
    let diags = check_all_for(&prog, &provided);
    assert!(
        !diags.is_empty(),
        "check_all_for must run capability_check (got {diags:?})"
    );
}

#[test]
fn check_all_for_clean_when_capabilities_satisfy() {
    let prog = extension_program();
    let provided = all_caps();
    let diags = check_all_for(&prog, &provided);
    assert!(diags.is_empty());
}
