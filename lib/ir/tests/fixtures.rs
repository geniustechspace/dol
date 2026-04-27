//! Per-`(Category, TargetKind)` fixtures.
//!
//! Builds at least one [`Operation`] for every meaningful pairing of
//! [`Category`](dol_ir::Category) and [`TargetKind`](dol_ir::TargetKind),
//! then asserts:
//!
//! - dispatch (`Operation::kind()` / `category()`) is correct,
//! - `required_capabilities()` agrees with the documented mapping,
//! - the operation round-trips through serde JSON when the `serde` feature
//!   is enabled.

use dol_expr::ids::NULL_NODE;
use dol_ir::operation::{
    Append, Delete, Describe, DescribeFacet, Insert, InsertSource, Probe, Query, Replace,
    ReplaceBody, SchemaOp, Update, Upsert,
};
use dol_ir::{
    CapabilityTag, Category, Locator, OpKind, Operation, SchemaBinding, SchemaRef, Symbol, Target,
    TargetKind,
};

fn t(kind: TargetKind) -> Target {
    Target::new(kind, Locator::new(Symbol::default()))
}

fn assert_basic(op: &Operation, kind: OpKind, cat: Category) {
    assert_eq!(op.kind(), kind, "kind mismatch for {op:?}");
    assert_eq!(op.category(), cat, "category mismatch for {op:?}");
}

#[test]
fn ddl_relation_create() {
    let op: Operation =
        SchemaOp::create_entity(t(TargetKind::Relation), SchemaRef::default(), false).into();
    assert_basic(&op, OpKind::Schema, Category::DDL);
}

#[test]
fn ddl_document_drop() {
    let op: Operation = SchemaOp::drop_(t(TargetKind::Document)).into();
    assert_basic(&op, OpKind::Schema, Category::DDL);
}

#[test]
fn ddl_blob_create_emits_blob_target_tag() {
    let op: Operation =
        SchemaOp::create_entity(t(TargetKind::Blob), SchemaRef::default(), true).into();
    assert_basic(&op, OpKind::Schema, Category::DDL);
    assert!(
        op.required_capabilities()
            .contains(&CapabilityTag::BLOB_TARGETS)
    );
}

#[test]
fn ddl_filetree_create_emits_file_tree_target_tag() {
    let op: Operation =
        SchemaOp::create_entity(t(TargetKind::FileTree), SchemaRef::default(), false).into();
    assert!(
        op.required_capabilities()
            .contains(&CapabilityTag::FILE_TREE_TARGETS)
    );
}

#[test]
fn ddl_stream_topic_create_emits_stream_target_tag() {
    let op: Operation =
        SchemaOp::create_entity(t(TargetKind::StreamTopic), SchemaRef::default(), false).into();
    assert!(
        op.required_capabilities()
            .contains(&CapabilityTag::STREAM_TARGETS)
    );
}

#[test]
fn dml_insert_relation() {
    let op: Operation = Insert {
        target: t(TargetKind::Relation).with_schema(SchemaBinding::Inferred),
        source: InsertSource::Bindings,
        returning: None,
    }
    .into();
    assert_basic(&op, OpKind::Insert, Category::DML);
    // No special target tags, no opaque schema tag.
    assert!(op.required_capabilities().is_empty());
}

#[test]
fn dml_insert_blob_emits_blob_targets() {
    let op: Operation = Insert {
        target: t(TargetKind::Blob),
        source: InsertSource::Bindings,
        returning: None,
    }
    .into();
    assert!(
        op.required_capabilities()
            .contains(&CapabilityTag::BLOB_TARGETS)
    );
}

#[test]
fn dml_replace_filetree_emits_replace_and_filetree_tags() {
    let op: Operation = Replace {
        target: t(TargetKind::FileTree),
        body: ReplaceBody::Bindings,
        filter: None,
    }
    .into();
    assert_basic(&op, OpKind::Replace, Category::DML);
    let req = op.required_capabilities();
    assert!(req.contains(&CapabilityTag::REPLACE_OP));
    assert!(req.contains(&CapabilityTag::FILE_TREE_TARGETS));
}

#[test]
fn dml_update_filetree_for_rename() {
    let op: Operation = Update {
        target: t(TargetKind::FileTree),
        node: NULL_NODE,
    }
    .into();
    assert_basic(&op, OpKind::Update, Category::DML);
}

#[test]
fn dml_delete_kv() {
    let op: Operation = Delete {
        target: t(TargetKind::KeyValue),
        node: NULL_NODE,
    }
    .into();
    assert_basic(&op, OpKind::Delete, Category::DML);
}

#[test]
fn dml_upsert_emits_merge_tag() {
    let op: Operation = Upsert {
        target: t(TargetKind::Relation),
        node: NULL_NODE,
    }
    .into();
    assert_basic(&op, OpKind::Upsert, Category::DML);
    assert!(op.required_capabilities().contains(&CapabilityTag::MERGE));
}

#[test]
fn dml_append_stream_emits_append_and_stream_tags() {
    let op: Operation = Append {
        target: t(TargetKind::StreamTopic),
        source: InsertSource::Bindings,
        partition_key: None,
    }
    .into();
    assert_basic(&op, OpKind::Append, Category::DML);
    let req = op.required_capabilities();
    assert!(req.contains(&CapabilityTag::APPEND_OP));
    assert!(req.contains(&CapabilityTag::STREAM_TARGETS));
}

#[test]
fn dql_query_blob_emits_blob_target() {
    let op: Operation = Query {
        target: t(TargetKind::Blob),
        node: None,
    }
    .into();
    assert_basic(&op, OpKind::Query, Category::DQL);
    assert!(
        op.required_capabilities()
            .contains(&CapabilityTag::BLOB_TARGETS)
    );
}

#[test]
fn dql_probe_api_resource() {
    let op: Operation = Probe {
        target: t(TargetKind::ApiResource),
        include_metadata: true,
    }
    .into();
    assert_basic(&op, OpKind::Probe, Category::DQL);
    let req = op.required_capabilities();
    assert!(req.contains(&CapabilityTag::PROBE_OP));
    assert!(req.contains(&CapabilityTag::API_TARGETS));
}

#[test]
fn dql_describe_relation_fields() {
    let op: Operation = Describe {
        target: t(TargetKind::Relation),
        facet: DescribeFacet::Fields,
    }
    .into();
    assert_basic(&op, OpKind::Describe, Category::DQL);
    assert!(
        op.required_capabilities()
            .contains(&CapabilityTag::DESCRIBE_OP)
    );
}

#[test]
fn opaque_schema_binding_emits_opaque_schema_tag() {
    use dol_ir::SchemaBinding;
    let op: Operation = Insert {
        target: t(TargetKind::Relation).with_schema(SchemaBinding::Opaque),
        source: InsertSource::Bindings,
        returning: None,
    }
    .into();
    assert!(
        op.required_capabilities()
            .contains(&CapabilityTag::OPAQUE_SCHEMA)
    );
}

// ── ACL / governance smoke tests ──────────────────────────────────────────

#[test]
fn acl_grant_select_on_relation() {
    use dol_ir::operation::Grant;
    use dol_ir::privilege::Privilege;
    use smallvec::smallvec;

    let op: Operation = Grant {
        privileges: smallvec![Privilege::Select],
        target: t(TargetKind::Relation),
        roles: smallvec![Symbol::new(0)],
        with_grant_option: false,
    }
    .into();
    assert_basic(&op, OpKind::Grant, Category::ACL);
}

#[test]
fn acl_revoke_custom_privilege_uses_symbol() {
    use dol_ir::operation::Revoke;
    use dol_ir::privilege::Privilege;
    use smallvec::smallvec;

    let op: Operation = Revoke {
        privileges: smallvec![Privilege::Custom(Symbol::new(7))],
        target: t(TargetKind::Relation),
        roles: smallvec![Symbol::new(0)],
        cascade: false,
    }
    .into();
    assert_basic(&op, OpKind::Revoke, Category::ACL);
}

#[test]
fn acl_policy_create_emits_policy_kind() {
    use dol_ir::operation::{PolicyOp, PolicyScope, StructuralVerb};

    let op: Operation = PolicyOp {
        verb: StructuralVerb::Create,
        target: t(TargetKind::Relation),
        name: Symbol::new(1),
        scope: PolicyScope::Read,
        using_expr: None,
        check_expr: None,
    }
    .into();
    assert_basic(&op, OpKind::Policy, Category::ACL);
}

#[test]
fn acl_mask_create_emits_mask_kind() {
    use dol_ir::operation::{MaskOp, StructuralVerb};
    use smallvec::smallvec;

    let op: Operation = MaskOp {
        verb: StructuralVerb::Create,
        target: t(TargetKind::Relation),
        name: Symbol::new(1),
        fields: smallvec![Symbol::new(2)],
        mask_expr: None,
    }
    .into();
    assert_basic(&op, OpKind::Mask, Category::ACL);
}

#[test]
fn acl_quota_create_emits_quota_kind() {
    use dol_ir::operation::{QuotaKind, QuotaOp, StructuralVerb};

    let op: Operation = QuotaOp {
        verb: StructuralVerb::Create,
        target: t(TargetKind::Relation),
        name: Symbol::new(1),
        kind: QuotaKind::Rate,
        limit: 1_000,
        role: None,
    }
    .into();
    assert_basic(&op, OpKind::Quota, Category::ACL);
}

#[test]
fn acl_audit_create_emits_audit_kind() {
    use dol_ir::operation::{AuditEvent, AuditOp, AuditSink, StructuralVerb};

    let op: Operation = AuditOp {
        verb: StructuralVerb::Create,
        target: t(TargetKind::Relation),
        name: Symbol::new(1),
        event: AuditEvent::Write,
        sink: AuditSink::Default,
    }
    .into();
    assert_basic(&op, OpKind::Audit, Category::ACL);
}

#[cfg(feature = "serde")]
#[test]
fn serde_round_trip_for_acl_and_governance() {
    use dol_ir::operation::{
        AuditEvent, AuditOp, AuditSink, Grant, MaskOp, PolicyOp, PolicyScope, QuotaKind, QuotaOp,
        Revoke, StructuralVerb,
    };
    use dol_ir::privilege::Privilege;
    use serde_json::{from_str, to_string};
    use smallvec::smallvec;

    let ops: Vec<Operation> = vec![
        Grant {
            privileges: smallvec![Privilege::Select, Privilege::Insert],
            target: t(TargetKind::Relation),
            roles: smallvec![Symbol::new(0)],
            with_grant_option: false,
        }
        .into(),
        Revoke {
            privileges: smallvec![Privilege::Custom(Symbol::new(7))],
            target: t(TargetKind::Relation),
            roles: smallvec![Symbol::new(0)],
            cascade: false,
        }
        .into(),
        PolicyOp {
            verb: StructuralVerb::Create,
            target: t(TargetKind::Relation),
            name: Symbol::new(1),
            scope: PolicyScope::Read,
            using_expr: None,
            check_expr: None,
        }
        .into(),
        MaskOp {
            verb: StructuralVerb::Create,
            target: t(TargetKind::Relation),
            name: Symbol::new(1),
            fields: smallvec![Symbol::new(2)],
            mask_expr: None,
        }
        .into(),
        QuotaOp {
            verb: StructuralVerb::Create,
            target: t(TargetKind::Relation),
            name: Symbol::new(1),
            kind: QuotaKind::Rate,
            limit: 100,
            role: None,
        }
        .into(),
        AuditOp {
            verb: StructuralVerb::Create,
            target: t(TargetKind::Relation),
            name: Symbol::new(1),
            event: AuditEvent::Write,
            sink: AuditSink::Default,
        }
        .into(),
        Append {
            target: t(TargetKind::StreamTopic),
            source: InsertSource::Bindings,
            partition_key: None,
        }
        .into(),
    ];
    for op in &ops {
        let s = to_string(op).expect("encode");
        let back: Operation = from_str(&s).expect("decode");
        assert_eq!(
            op.kind(),
            back.kind(),
            "kind mismatch round-tripping {op:?}"
        );
        assert_eq!(op, &back, "value mismatch round-tripping {op:?}");
    }
}

#[test]
fn operation_size_within_budget() {
    assert!(
        core::mem::size_of::<Operation>() <= 64,
        "Operation must fit in 64 bytes (got {})",
        core::mem::size_of::<Operation>()
    );
}

#[test]
fn operation_is_send_sync_static() {
    fn assert_send_sync_static<T: Send + Sync + 'static>() {}
    assert_send_sync_static::<Operation>();
}

// ── Serde round-trips ─────────────────────────────────────────────────────

#[cfg(feature = "serde")]
#[test]
fn serde_round_trip_for_each_category() {
    use serde_json::{from_str, to_string};

    let ops: Vec<Operation> = vec![
        SchemaOp::create_entity(t(TargetKind::Relation), SchemaRef::default(), false).into(),
        Insert {
            target: t(TargetKind::Document),
            source: InsertSource::Bindings,
            returning: None,
        }
        .into(),
        Query {
            target: t(TargetKind::Blob),
            node: None,
        }
        .into(),
        Append {
            target: t(TargetKind::StreamTopic),
            source: InsertSource::Bindings,
            partition_key: None,
        }
        .into(),
        Replace {
            target: t(TargetKind::FileTree),
            body: ReplaceBody::Bindings,
            filter: None,
        }
        .into(),
    ];
    for op in &ops {
        let s = to_string(op).expect("encode");
        let back: Operation = from_str(&s).expect("decode");
        assert_eq!(op.kind(), back.kind());
        assert_eq!(op.category(), back.category());
        assert_eq!(
            op.primary_target().map(|t| &t.kind),
            back.primary_target().map(|t| &t.kind)
        );
    }
}
