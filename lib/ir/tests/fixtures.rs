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
    Category, CapabilityTag, Locator, OpKind, Operation, SchemaBinding, SchemaRef, Symbol,
    Target, TargetKind,
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
    let op: Operation = Insert {
        target: t(TargetKind::Relation), // default binding is Opaque
        source: InsertSource::Bindings,
        returning: None,
    }
    .into();
    assert!(
        op.required_capabilities()
            .contains(&CapabilityTag::OPAQUE_SCHEMA)
    );
}

// ── Static invariants ─────────────────────────────────────────────────────

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
