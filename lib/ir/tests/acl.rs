//! ACL governance object tests for IR.
//!
//! Exercises the four nouns (`Policy`, `Mask`, `Quota`, `Audit`) plus the
//! ACL verbs (`Grant`, `Revoke`) and confirms they:
//! - dispatch through `OpKind` and `Category::ACL`,
//! - report the correct capability tags via `required_capabilities()`,
//! - construct cleanly through `From<Payload> for Operation`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use dol_ir::operation::{
    AuditEvent, AuditOp, AuditSink, Grant, MaskOp, PolicyOp, PolicyScope, QuotaKind, QuotaOp,
    Revoke, StructuralVerb,
};
use dol_ir::capabilities::CapabilityTag;
use dol_ir::operation::{Category, OpKind, Operation};
use dol_ir::privilege::Privilege;
use dol_ir::target::{Locator, Symbol, Target, TargetKind};
use smallvec::smallvec;

fn rel(name_id: u32) -> Target {
    Target::new(
        TargetKind::Relation,
        Locator::new(Symbol::from_hash(name_id)),
    )
}

#[test]
fn policy_create_dispatches_correctly() {
    let op: Operation = PolicyOp {
        verb: StructuralVerb::Create,
        target: rel(0),
        name: Symbol::from_hash(1),
        scope: PolicyScope::Read,
        using_expr: None,
        check_expr: None,
    }
    .into();
    assert_eq!(op.kind(), OpKind::Policy);
    assert_eq!(op.category(), Category::ACL);
    assert!(
        op.required_capabilities()
            .contains(&CapabilityTag::POLICIES)
    );
}

#[test]
fn mask_emits_mask_policies_tag() {
    let op: Operation = MaskOp {
        verb: StructuralVerb::Create,
        target: rel(0),
        name: Symbol::from_hash(1),
        fields: smallvec![Symbol::from_hash(2)],
        mask_expr: None,
    }
    .into();
    assert_eq!(op.kind(), OpKind::Mask);
    assert_eq!(op.category(), Category::ACL);
    assert!(
        op.required_capabilities()
            .contains(&CapabilityTag::MASK_POLICIES)
    );
}

#[test]
fn quota_emits_quotas_tag_and_carries_kind() {
    let op: Operation = QuotaOp {
        verb: StructuralVerb::Create,
        target: rel(0),
        name: Symbol::from_hash(1),
        kind: QuotaKind::Storage,
        limit: 1_000_000,
        role: None,
    }
    .into();
    assert_eq!(op.kind(), OpKind::Quota);
    assert_eq!(op.category(), Category::ACL);
    assert!(op.required_capabilities().contains(&CapabilityTag::QUOTAS));
    if let Operation::Quota(q) = &op {
        assert_eq!(q.kind, QuotaKind::Storage);
        assert_eq!(q.limit, 1_000_000);
    } else {
        panic!("expected Quota");
    }
}

#[test]
fn audit_emits_audit_tag_and_carries_event() {
    let op: Operation = AuditOp {
        verb: StructuralVerb::Create,
        target: rel(0),
        name: Symbol::from_hash(1),
        event: AuditEvent::Write,
        sink: AuditSink::Default,
    }
    .into();
    assert_eq!(op.kind(), OpKind::Audit);
    assert_eq!(op.category(), Category::ACL);
    assert!(op.required_capabilities().contains(&CapabilityTag::AUDIT));
    if let Operation::Audit(a) = &op {
        assert_eq!(a.event, AuditEvent::Write);
    } else {
        panic!("expected Audit");
    }
}

#[test]
fn grant_dispatches_to_acl_verb() {
    let op: Operation = Grant {
        privileges: smallvec![Privilege::Select, Privilege::Update],
        target: rel(0),
        roles: smallvec![Symbol::from_hash(1)],
        with_grant_option: false,
    }
    .into();
    assert_eq!(op.kind(), OpKind::Grant);
    assert_eq!(op.category(), Category::ACL);
}

#[test]
fn revoke_dispatches_to_acl_verb_with_cascade() {
    let op: Operation = Revoke {
        privileges: smallvec![Privilege::All],
        target: rel(0),
        roles: smallvec![Symbol::from_hash(1)],
        cascade: true,
    }
    .into();
    assert_eq!(op.kind(), OpKind::Revoke);
    if let Operation::Revoke(r) = &op {
        assert!(r.cascade);
    } else {
        panic!("expected Revoke");
    }
}

#[test]
fn truncate_targets_data_kinds_only_at_caller_discretion() {
    use dol_ir::operation::SchemaOp;
    // Truncate against a Relation: legitimate.
    let ok: Operation = SchemaOp::truncate(rel(0)).into();
    assert_eq!(ok.kind(), OpKind::Schema);
    // Truncate against a Virtual: capability layer rejects (RFC §1).
    // We only assert that the operation itself constructs; the actual
    // rejection is `dol-check`'s job and is exercised in its tests.
    let on_virtual: Operation = SchemaOp::truncate(Target::new(
        TargetKind::Virtual,
        Locator::new(Symbol::default()),
    ))
    .into();
    assert_eq!(on_virtual.kind(), OpKind::Schema);
}
