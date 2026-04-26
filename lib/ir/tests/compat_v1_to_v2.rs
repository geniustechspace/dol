//! v1 → v2 compat decode tests.
//!
//! Every entry in the RFC's "Mapping Old → New" table has a row here.

use dol_ir::compat::statement::statement_to_operation;
use dol_ir::definition::{DefineEntity, DefineLookup, DefineType, DropEntity, DropLookup, DropType};
use dol_ir::operation::{Operation, OpKind, ReplaceBody, TxOp};
use dol_ir::storage::{
    GetObject, ListObjects, MoveFile, ObjectSource, PutObject, ReadFile, WriteFile,
};
use dol_ir::transaction::Transaction;
use dol_ir::{
    DefinePolicy, Grant, PolicyAction, Privilege, Revoke, Statement, StatementExtension,
    TargetKind,
};

fn check_kind_and_target(stmt: Statement, kind: OpKind, target: Option<TargetKind>) {
    let op = statement_to_operation(&stmt);
    assert_eq!(op.kind(), kind, "kind mismatch for stmt={stmt:?}");
    assert_eq!(
        op.primary_target().map(|t| t.kind.clone()),
        target,
        "target mismatch for stmt={stmt:?}",
    );
}

#[test]
fn query_maps_direct() {
    use dol_expr::expr::QueryNode;
    use smallvec::smallvec;
    let q = Statement::Query(Box::new(QueryNode {
        from: 0,
        alias: None,
        joins: Default::default(),
        filter: 0,
        columns: smallvec![],
        group_by: Default::default(),
        having: 0,
        order_by: Default::default(),
        limit: None,
        offset: None,
        lock: None,
    }));
    check_kind_and_target(q, OpKind::Query, Some(TargetKind::Relation));
}

#[test]
fn define_entity_maps_to_schema_create() {
    let de = Statement::from(DefineEntity {
        name: "users".into(),
        namespace: None,
        fields: vec![],
        constraints: vec![],
        if_not_exists: false,
    });
    check_kind_and_target(de, OpKind::Schema, Some(TargetKind::Relation));
}

#[test]
fn drop_entity_maps_to_schema_drop() {
    let s = Statement::from(DropEntity {
        target: dol_ir::EntityRef::new("users"),
        if_exists: false,
        cascade: false,
    });
    check_kind_and_target(s, OpKind::Schema, Some(TargetKind::Relation));
}

#[test]
fn define_lookup_maps_to_lookup() {
    let s = Statement::from(DefineLookup {
        name: "ix".into(),
        target: dol_ir::EntityRef::new("users"),
        columns: vec!["id".into()],
        unique: false,
        if_not_exists: false,
        concurrently: false,
        method: Some(dol_ir::LookupMethod::Equality),
        where_clause: None,
    });
    check_kind_and_target(s, OpKind::Lookup, Some(TargetKind::Relation));
}

#[test]
fn drop_lookup_maps_to_lookup_drop() {
    let s = Statement::from(DropLookup {
        name: "ix".into(),
        if_exists: false,
        concurrently: false,
        cascade: false,
    });
    check_kind_and_target(s, OpKind::Lookup, Some(TargetKind::Relation));
}

#[test]
fn define_type_maps_to_schema_create_type() {
    let s = Statement::from(DefineType {
        name: "status".into(),
        namespace: None,
        variants: vec!["ok".into()],
    });
    check_kind_and_target(s, OpKind::Schema, Some(TargetKind::Virtual));
}

#[test]
fn drop_type_maps_to_schema_drop() {
    let s = Statement::from(DropType {
        name: "status".into(),
        if_exists: false,
    });
    check_kind_and_target(s, OpKind::Schema, Some(TargetKind::Virtual));
}

#[test]
fn grant_maps_to_grant_v2() {
    let s = Statement::from(Grant {
        privilege: Privilege::Select,
        on_target: "users".into(),
        to_role: "reader".into(),
    });
    check_kind_and_target(s, OpKind::Grant, Some(TargetKind::Relation));
}

#[test]
fn revoke_maps_to_revoke_v2() {
    let s = Statement::from(Revoke {
        privilege: Privilege::Update,
        on_target: "users".into(),
        from_role: "writer".into(),
    });
    check_kind_and_target(s, OpKind::Revoke, Some(TargetKind::Relation));
}

#[test]
fn define_policy_maps_to_policy() {
    let s = Statement::from(DefinePolicy {
        name: "p".into(),
        on_model: "users".into(),
        action: PolicyAction::Read,
        using_expr: None,
        check_expr: None,
    });
    check_kind_and_target(s, OpKind::Policy, Some(TargetKind::Relation));
}

#[test]
fn transaction_maps_to_tx() {
    let s = Statement::Transaction(Box::new(Transaction::Commit));
    let op = statement_to_operation(&s);
    assert_eq!(op.kind(), OpKind::Tx);
    assert!(matches!(op, Operation::Tx(ref b) if matches!(**b, TxOp::Commit)));
}

#[test]
fn put_object_maps_to_insert_blob() {
    let s = Statement::PutObject(Box::new(PutObject {
        bucket: "b".into(),
        key: "k".into(),
        source: ObjectSource::FromBytes,
        content_type: None,
        metadata: vec![],
    }));
    check_kind_and_target(s, OpKind::Insert, Some(TargetKind::Blob));
}

#[test]
fn get_object_maps_to_query_blob() {
    let s = Statement::GetObject(Box::new(GetObject {
        bucket: "b".into(),
        key: "k".into(),
    }));
    check_kind_and_target(s, OpKind::Query, Some(TargetKind::Blob));
}

#[test]
fn list_objects_maps_to_query_blob() {
    let s = Statement::ListObjects(Box::new(ListObjects {
        bucket: "b".into(),
        prefix: None,
        limit: None,
        continuation_token: None,
    }));
    check_kind_and_target(s, OpKind::Query, Some(TargetKind::Blob));
}

#[test]
fn read_file_maps_to_query_filetree() {
    let s = Statement::ReadFile(Box::new(ReadFile {
        path: "/x".into(),
        encoding: None,
    }));
    check_kind_and_target(s, OpKind::Query, Some(TargetKind::FileTree));
}

#[test]
fn write_file_maps_to_replace_filetree() {
    let s = Statement::WriteFile(Box::new(WriteFile {
        path: "/x".into(),
        source: ObjectSource::FromBytes,
        create_dirs: false,
    }));
    let op = statement_to_operation(&s);
    assert_eq!(op.kind(), OpKind::Replace);
    assert_eq!(op.primary_target().unwrap().kind, TargetKind::FileTree);
    if let Operation::Replace(r) = &op {
        assert!(matches!(r.body, ReplaceBody::Bindings));
    } else {
        panic!("expected Replace");
    }
}

#[test]
fn move_file_maps_to_update_filetree() {
    let s = Statement::MoveFile(Box::new(MoveFile {
        from: "/a".into(),
        to: "/b".into(),
    }));
    check_kind_and_target(s, OpKind::Update, Some(TargetKind::FileTree));
}

#[test]
fn raw_maps_to_raw_when_feature_enabled() {
    let s = Statement::Raw("SELECT 1".into());
    let op = statement_to_operation(&s);
    #[cfg(feature = "raw")]
    assert_eq!(op.kind(), OpKind::Raw);
    #[cfg(not(feature = "raw"))]
    assert_eq!(op.kind(), OpKind::Extension);
}

#[test]
fn extension_maps_to_extension() {
    let s = Statement::from(StatementExtension {
        id: "test/x".into(),
        payload: vec![1, 2, 3],
    });
    let op = statement_to_operation(&s);
    assert_eq!(op.kind(), OpKind::Extension);
}

#[test]
fn alter_entity_maps_to_schema_alter() {
    use dol_ir::AlterEntity;
    let s = Statement::from(AlterEntity {
        target: dol_ir::EntityRef::new("users"),
        actions: vec![],
    });
    check_kind_and_target(s, OpKind::Schema, Some(TargetKind::Relation));
}
