use super::*;
use super::control::{DefinePolicyIR, GrantIR, PolicyAction, Privilege, RevokeIR};
use super::definition::{
    AlterAction, AlterEntityIR, DefineEntityIR, DefineIndexIR, DefineTypeIR, DropEntityIR,
    DropIndexIR, DropTypeIR, FieldDef, IndexMethod, OwnedForeignKeyRef,
};
use super::mutation::{InsertIR, InsertSelectIR, RemoveIR, UpdateIR, UpsertIR};
use super::query::{CompoundQueryIR, JoinIR, JoinType, LockMode, OffsetLimit, QueryIR, SetOpKind};
use super::storage::{
    GetObjectIR, ListObjectsIR, MoveFileIR, ObjectSource, PutObjectIR, ReadFileIR, WriteFileIR,
};
use super::transaction::TransactionIR;
use crate::types::DataType;
use crate::constraint::FkAction;

// -- helpers --

fn entity_ref(name: &str) -> EntityRef {
    EntityRef {
        name: name.to_string(),
        namespace: None,
        alias: None,
    }
}

fn entity_ref_full(name: &str, ns: &str, alias: &str) -> EntityRef {
    EntityRef {
        name: name.to_string(),
        namespace: Some(ns.to_string()),
        alias: Some(alias.to_string()),
    }
}

fn simple_query_ir() -> QueryIR<'static> {
    QueryIR {
        source: entity_ref("users"),
        projections: vec![],
        joins: vec![],
        filters: vec![],
        group_by: vec![],
        having: vec![],
        order_by: vec![],
        offset: None,
        limit: None,
        distinct: false,
        distinct_on: vec![],
        lock_mode: None,
    }
}

// ======================================================================
// 1. EntityRef
// ======================================================================

#[test]
fn model_ref_construction_minimal() {
    let mr = entity_ref("users");
    assert_eq!(mr.name, "users");
    assert!(mr.namespace.is_none());
    assert!(mr.alias.is_none());
}

#[test]
fn model_ref_construction_full() {
    let mr = entity_ref_full("users", "public", "u");
    assert_eq!(mr.name, "users");
    assert_eq!(mr.namespace.as_deref(), Some("public"));
    assert_eq!(mr.alias.as_deref(), Some("u"));
}

#[test]
fn model_ref_debug() {
    let mr = entity_ref("orders");
    let dbg = format!("{:?}", mr);
    assert!(dbg.contains("orders"));
}

#[test]
fn model_ref_clone() {
    let mr = entity_ref_full("products", "shop", "p");
    let cloned = mr.clone();
    assert_eq!(cloned.name, mr.name);
    assert_eq!(cloned.namespace, mr.namespace);
    assert_eq!(cloned.alias, mr.alias);
}

// ======================================================================
// 2. Statement construction
// ======================================================================

#[test]
fn statement_define_model() {
    let stmt = Statement::DefineEntity(Box::new(DefineEntityIR {
        name: "users".to_string(),
        namespace: None,
        fields: vec![],
        constraints: vec![],
        if_not_exists: false,
    }));
    assert!(matches!(stmt, Statement::DefineEntity(_)));
}

#[test]
fn statement_drop_model() {
    let stmt = Statement::DropEntity(DropEntityIR {
        target: entity_ref("users"),
        if_exists: true,
        cascade: false,
    });
    assert!(matches!(stmt, Statement::DropEntity(_)));
}

#[test]
fn statement_insert() {
    let stmt = Statement::Insert(InsertIR {
        target: entity_ref("users"),
        fields: vec!["name".to_string()],
        row_count: 1,
        returning: vec![],
    });
    assert!(matches!(stmt, Statement::Insert(_)));
}

#[test]
fn statement_query() {
    let stmt = Statement::Query(Box::new(simple_query_ir()));
    assert!(matches!(stmt, Statement::Query(_)));
}

#[test]
fn statement_transaction() {
    let stmt = Statement::Transaction(TransactionIR::Begin);
    assert!(matches!(stmt, Statement::Transaction(_)));
}

#[test]
fn statement_grant() {
    let stmt = Statement::Grant(GrantIR {
        privilege: Privilege::Select,
        on_target: "users".to_string(),
        to_role: "reader".to_string(),
    });
    assert!(matches!(stmt, Statement::Grant(_)));
}

#[test]
fn statement_put_object() {
    let stmt = Statement::PutObject(PutObjectIR {
        key: "file.txt".to_string(),
        source: ObjectSource::FromBytes,
        bucket: "uploads".to_string(),
        content_type: None,
        metadata: vec![],
    });
    assert!(matches!(stmt, Statement::PutObject(_)));
}

#[test]
fn statement_read_file() {
    let stmt = Statement::ReadFile(ReadFileIR {
        path: "/data/input.csv".to_string(),
        encoding: Some("utf-8".to_string()),
    });
    assert!(matches!(stmt, Statement::ReadFile(_)));
}

#[test]
fn statement_clone() {
    let stmt = Statement::Transaction(TransactionIR::Commit);
    let cloned = stmt.clone();
    assert!(matches!(
        cloned,
        Statement::Transaction(TransactionIR::Commit)
    ));
}

#[test]
fn statement_debug() {
    let stmt = Statement::Transaction(TransactionIR::Rollback);
    let dbg = format!("{:?}", stmt);
    assert!(dbg.contains("Rollback"));
}

// ======================================================================
// 3. SqlOutput
// ======================================================================

#[test]
fn sql_output_construction() {
    let out = SqlOutput {
        sql: "SELECT * FROM users WHERE id = $1".to_string(),
        param_count: 1,
    };
    assert_eq!(out.sql, "SELECT * FROM users WHERE id = $1");
    assert_eq!(out.param_count, 1);
}

#[test]
fn sql_output_zero_params() {
    let out = SqlOutput {
        sql: "SELECT 1".to_string(),
        param_count: 0,
    };
    assert_eq!(out.param_count, 0);
}

#[test]
fn sql_output_clone_and_debug() {
    let out = SqlOutput {
        sql: "INSERT INTO t VALUES ($1)".to_string(),
        param_count: 1,
    };
    let cloned = out.clone();
    assert_eq!(cloned.sql, out.sql);
    let dbg = format!("{:?}", out);
    assert!(dbg.contains("INSERT"));
}

// ======================================================================
// 4. KvOp
// ======================================================================

#[test]
fn kv_op_get_eq() {
    assert_eq!(KvOp::Get, KvOp::Get);
}

#[test]
fn kv_op_put_eq() {
    assert_eq!(KvOp::Put, KvOp::Put);
}

#[test]
fn kv_op_delete_eq() {
    assert_eq!(KvOp::Delete, KvOp::Delete);
}

#[test]
fn kv_op_list_eq_no_filters() {
    assert_eq!(
        KvOp::List {
            prefix: None,
            limit: None,
        },
        KvOp::List {
            prefix: None,
            limit: None,
        }
    );
}

#[test]
fn kv_op_list_eq_with_prefix_and_limit() {
    assert_eq!(
        KvOp::List {
            prefix: Some("user:".to_string()),
            limit: Some(100),
        },
        KvOp::List {
            prefix: Some("user:".to_string()),
            limit: Some(100),
        }
    );
}

#[test]
fn kv_op_different_variants_ne() {
    assert_ne!(KvOp::Get, KvOp::Put);
    assert_ne!(KvOp::Put, KvOp::Delete);
    assert_ne!(
        KvOp::Get,
        KvOp::List {
            prefix: None,
            limit: None
        }
    );
}

// ======================================================================
// 5. StorageOp
// ======================================================================

#[test]
fn storage_op_put_object_eq() {
    let a = StorageOp::PutObject {
        bucket: "b".to_string(),
        key: "k".to_string(),
        content_type: Some("text/plain".to_string()),
    };
    let b = StorageOp::PutObject {
        bucket: "b".to_string(),
        key: "k".to_string(),
        content_type: Some("text/plain".to_string()),
    };
    assert_eq!(a, b);
}

#[test]
fn storage_op_get_object_eq() {
    assert_eq!(
        StorageOp::GetObject {
            bucket: "bkt".to_string(),
            key: "obj".to_string(),
        },
        StorageOp::GetObject {
            bucket: "bkt".to_string(),
            key: "obj".to_string(),
        }
    );
}

#[test]
fn storage_op_list_objects_eq() {
    assert_eq!(
        StorageOp::ListObjects {
            bucket: "b".to_string(),
            prefix: Some("img/".to_string()),
            limit: Some(50),
        },
        StorageOp::ListObjects {
            bucket: "b".to_string(),
            prefix: Some("img/".to_string()),
            limit: Some(50),
        }
    );
}

#[test]
fn storage_op_delete_object_eq() {
    assert_eq!(
        StorageOp::DeleteObject {
            bucket: "b".to_string(),
            key: "old".to_string(),
        },
        StorageOp::DeleteObject {
            bucket: "b".to_string(),
            key: "old".to_string(),
        }
    );
}

#[test]
fn storage_op_different_variants_ne() {
    let put = StorageOp::PutObject {
        bucket: "b".to_string(),
        key: "k".to_string(),
        content_type: None,
    };
    let get = StorageOp::GetObject {
        bucket: "b".to_string(),
        key: "k".to_string(),
    };
    assert_ne!(put, get);
}

// ======================================================================
// 6. BackendError
// ======================================================================

#[test]
fn backend_error_unsupported_display() {
    let err = BackendError::Unsupported("KV insert".to_string());
    assert_eq!(format!("{}", err), "unsupported: KV insert");
}

#[test]
fn backend_error_render_display() {
    let err = BackendError::RenderError("bad SQL".to_string());
    assert_eq!(format!("{}", err), "render error: bad SQL");
}

#[test]
fn backend_error_implements_error_trait() {
    let err = BackendError::Unsupported("x".to_string());
    let _: &dyn std::error::Error = &err;
}

#[test]
fn backend_error_debug() {
    let err = BackendError::RenderError("oops".to_string());
    let dbg = format!("{:?}", err);
    assert!(dbg.contains("RenderError"));
    assert!(dbg.contains("oops"));
}

// ======================================================================
// 7. RenderedOutput
// ======================================================================

#[test]
fn rendered_output_sql() {
    let out = RenderedOutput::Sql(SqlOutput {
        sql: "SELECT 1".to_string(),
        param_count: 0,
    });
    assert!(matches!(out, RenderedOutput::Sql(_)));
}

#[test]
fn rendered_output_key_value() {
    let out = RenderedOutput::KeyValue(KvOutput {
        operation: KvOp::Get,
        key: "user:42".to_string(),
        metadata: vec![],
    });
    assert!(matches!(out, RenderedOutput::KeyValue(_)));
}

#[test]
fn rendered_output_storage() {
    let out = RenderedOutput::Storage(StorageOutput {
        operation: StorageOp::GetObject {
            bucket: "assets".to_string(),
            key: "logo.png".to_string(),
        },
    });
    assert!(matches!(out, RenderedOutput::Storage(_)));
}

// ======================================================================
// 8. FieldDef builder
// ======================================================================

#[test]
fn field_def_new_defaults() {
    let f = FieldDef::new("id", DataType::Uuid);
    assert_eq!(f.name, "id");
    assert!(!f.primary_key);
    assert!(!f.nullable);
    assert!(!f.unique);
    assert!(!f.indexed);
    assert!(f.default_expr.is_none());
    assert!(f.references.is_none());
    assert!(f.check.is_none());
    assert!(f.comment.is_none());
    assert!(f.collation.is_none());
    assert!(f.generated.is_none());
}

#[test]
fn field_def_primary_key() {
    let f = FieldDef::new("id", DataType::Int32).primary_key();
    assert!(f.primary_key);
}

#[test]
fn field_def_nullable() {
    let f = FieldDef::new("bio", DataType::Text).nullable();
    assert!(f.nullable);
}

#[test]
fn field_def_optional_is_nullable() {
    let f = FieldDef::new("bio", DataType::Text).optional();
    assert!(f.nullable);
}

#[test]
fn field_def_required_is_not_nullable() {
    let f = FieldDef::new("email", DataType::Text).required();
    assert!(!f.nullable);
}

#[test]
fn field_def_default() {
    let f = FieldDef::new("status", DataType::Text).default("'active'");
    assert_eq!(f.default_expr.as_deref(), Some("'active'"));
}

#[test]
fn field_def_unique() {
    let f = FieldDef::new("email", DataType::Text).unique();
    assert!(f.unique);
}

#[test]
fn field_def_check() {
    let f = FieldDef::new("age", DataType::Int32).check("age > 0");
    assert_eq!(f.check.as_deref(), Some("age > 0"));
}

#[test]
fn field_def_comment() {
    let f = FieldDef::new("id", DataType::Uuid).comment("Primary key");
    assert_eq!(f.comment.as_deref(), Some("Primary key"));
}

#[test]
fn field_def_collation() {
    let f = FieldDef::new("name", DataType::Text).collation("en_US.utf8");
    assert_eq!(f.collation.as_deref(), Some("en_US.utf8"));
}

#[test]
fn field_def_index() {
    let f = FieldDef::new("email", DataType::Text).index();
    assert!(f.indexed);
}

#[test]
fn field_def_generated_stored() {
    let f = FieldDef::new("full_name", DataType::Text)
        .generated_stored("first_name || ' ' || last_name");
    assert!(f.generated.is_some());
    let (kind, expr) = f.generated.unwrap();
    assert!(matches!(kind, crate::constraint::GeneratedKind::Stored));
    assert_eq!(expr, "first_name || ' ' || last_name");
}

#[test]
fn field_def_generated_virtual() {
    let f = FieldDef::new("age_group", DataType::Text).generated_virtual("age_bucket(age)");
    assert!(f.generated.is_some());
    let (kind, expr) = f.generated.unwrap();
    assert!(matches!(kind, crate::constraint::GeneratedKind::Virtual));
    assert_eq!(expr, "age_bucket(age)");
}

#[test]
fn field_def_references() {
    let fk = OwnedForeignKeyRef::new("users", "id");
    let f = FieldDef::new("user_id", DataType::Uuid).references(fk);
    assert!(f.references.is_some());
    let r = f.references.unwrap();
    assert_eq!(r.table, "users");
    assert_eq!(r.column, "id");
}

#[test]
fn field_def_builder_chain() {
    let f = FieldDef::new("email", DataType::Text)
        .unique()
        .nullable()
        .index()
        .comment("user email")
        .check("email LIKE '%@%'");
    assert!(f.unique);
    assert!(f.nullable);
    assert!(f.indexed);
    assert_eq!(f.comment.as_deref(), Some("user email"));
    assert_eq!(f.check.as_deref(), Some("email LIKE '%@%'"));
}

// ======================================================================
// 9. OwnedForeignKeyRef
// ======================================================================

#[test]
fn owned_fk_ref_new_defaults() {
    let fk = OwnedForeignKeyRef::new("users", "id");
    assert_eq!(fk.table, "users");
    assert_eq!(fk.column, "id");
    assert_eq!(fk.on_delete, FkAction::NoAction);
    assert_eq!(fk.on_update, FkAction::NoAction);
}

#[test]
fn owned_fk_ref_on_delete() {
    let fk = OwnedForeignKeyRef::new("users", "id").on_delete(FkAction::Cascade);
    assert_eq!(fk.on_delete, FkAction::Cascade);
    assert_eq!(fk.on_update, FkAction::NoAction);
}

#[test]
fn owned_fk_ref_on_update() {
    let fk = OwnedForeignKeyRef::new("users", "id").on_update(FkAction::SetNull);
    assert_eq!(fk.on_delete, FkAction::NoAction);
    assert_eq!(fk.on_update, FkAction::SetNull);
}

#[test]
fn owned_fk_ref_both_actions() {
    let fk = OwnedForeignKeyRef::new("orgs", "org_id")
        .on_delete(FkAction::Restrict)
        .on_update(FkAction::Cascade);
    assert_eq!(fk.on_delete, FkAction::Restrict);
    assert_eq!(fk.on_update, FkAction::Cascade);
}

// ======================================================================
// 10. IndexMethod
// ======================================================================

#[test]
fn index_method_equality() {
    assert_eq!(IndexMethod::BTree, IndexMethod::BTree);
    assert_eq!(IndexMethod::Hash, IndexMethod::Hash);
    assert_eq!(IndexMethod::Gin, IndexMethod::Gin);
    assert_eq!(IndexMethod::Gist, IndexMethod::Gist);
    assert_eq!(IndexMethod::SpGist, IndexMethod::SpGist);
    assert_eq!(IndexMethod::Brin, IndexMethod::Brin);
}

#[test]
fn index_method_inequality() {
    assert_ne!(IndexMethod::BTree, IndexMethod::Hash);
    assert_ne!(IndexMethod::Gin, IndexMethod::Gist);
}

// ======================================================================
// 11. Privilege
// ======================================================================

#[test]
fn privilege_equality_all_variants() {
    assert_eq!(Privilege::Select, Privilege::Select);
    assert_eq!(Privilege::Insert, Privilege::Insert);
    assert_eq!(Privilege::Update, Privilege::Update);
    assert_eq!(Privilege::Delete, Privilege::Delete);
    assert_eq!(Privilege::All, Privilege::All);
    assert_eq!(Privilege::Usage, Privilege::Usage);
    assert_eq!(Privilege::Create, Privilege::Create);
    assert_eq!(Privilege::Connect, Privilege::Connect);
}

#[test]
fn privilege_custom_equality() {
    assert_eq!(
        Privilege::Custom("EXECUTE".to_string()),
        Privilege::Custom("EXECUTE".to_string())
    );
}

#[test]
fn privilege_custom_inequality() {
    assert_ne!(
        Privilege::Custom("EXECUTE".to_string()),
        Privilege::Custom("TRUNCATE".to_string())
    );
}

#[test]
fn privilege_different_variants_ne() {
    assert_ne!(Privilege::Select, Privilege::Insert);
    assert_ne!(Privilege::All, Privilege::Custom("ALL".to_string()));
}

// ======================================================================
// 12. PolicyAction
// ======================================================================

#[test]
fn policy_action_equality() {
    assert_eq!(PolicyAction::Read, PolicyAction::Read);
    assert_eq!(PolicyAction::Write, PolicyAction::Write);
    assert_eq!(PolicyAction::All, PolicyAction::All);
}

#[test]
fn policy_action_inequality() {
    assert_ne!(PolicyAction::Read, PolicyAction::Write);
    assert_ne!(PolicyAction::Write, PolicyAction::All);
    assert_ne!(PolicyAction::Read, PolicyAction::All);
}

// ======================================================================
// 13. LockMode
// ======================================================================

#[test]
fn lock_mode_equality() {
    assert_eq!(LockMode::ForUpdate, LockMode::ForUpdate);
    assert_eq!(LockMode::ForShare, LockMode::ForShare);
    assert_eq!(LockMode::ForUpdateNoWait, LockMode::ForUpdateNoWait);
    assert_eq!(LockMode::ForUpdateSkipLocked, LockMode::ForUpdateSkipLocked);
    assert_eq!(LockMode::ForShareNoWait, LockMode::ForShareNoWait);
    assert_eq!(LockMode::ForShareSkipLocked, LockMode::ForShareSkipLocked);
}

#[test]
fn lock_mode_inequality() {
    assert_ne!(LockMode::ForUpdate, LockMode::ForShare);
    assert_ne!(LockMode::ForUpdateNoWait, LockMode::ForShareNoWait);
}

// ======================================================================
// 14. JoinType
// ======================================================================

#[test]
fn join_type_equality() {
    assert_eq!(JoinType::Inner, JoinType::Inner);
    assert_eq!(JoinType::Left, JoinType::Left);
    assert_eq!(JoinType::Right, JoinType::Right);
    assert_eq!(JoinType::Full, JoinType::Full);
    assert_eq!(JoinType::Cross, JoinType::Cross);
}

#[test]
fn join_type_inequality() {
    assert_ne!(JoinType::Inner, JoinType::Left);
    assert_ne!(JoinType::Left, JoinType::Right);
    assert_ne!(JoinType::Full, JoinType::Cross);
}

// ======================================================================
// 15. SetOpKind
// ======================================================================

#[test]
fn set_op_kind_equality() {
    assert_eq!(SetOpKind::Union, SetOpKind::Union);
    assert_eq!(SetOpKind::UnionAll, SetOpKind::UnionAll);
    assert_eq!(SetOpKind::Intersect, SetOpKind::Intersect);
    assert_eq!(SetOpKind::IntersectAll, SetOpKind::IntersectAll);
    assert_eq!(SetOpKind::Except, SetOpKind::Except);
    assert_eq!(SetOpKind::ExceptAll, SetOpKind::ExceptAll);
}

#[test]
fn set_op_kind_inequality() {
    assert_ne!(SetOpKind::Union, SetOpKind::UnionAll);
    assert_ne!(SetOpKind::Intersect, SetOpKind::Except);
}

// ======================================================================
// 16. KvOutput
// ======================================================================

#[test]
fn kv_output_construction_empty_metadata() {
    let out = KvOutput {
        operation: KvOp::Get,
        key: "user:1".to_string(),
        metadata: vec![],
    };
    assert_eq!(out.operation, KvOp::Get);
    assert_eq!(out.key, "user:1");
    assert!(out.metadata.is_empty());
}

#[test]
fn kv_output_construction_with_metadata() {
    let out = KvOutput {
        operation: KvOp::Put,
        key: "session:abc".to_string(),
        metadata: vec![
            ("ttl".to_string(), "3600".to_string()),
            ("type".to_string(), "session".to_string()),
        ],
    };
    assert_eq!(out.operation, KvOp::Put);
    assert_eq!(out.key, "session:abc");
    assert_eq!(out.metadata.len(), 2);
    assert_eq!(out.metadata[0], ("ttl".to_string(), "3600".to_string()));
}

#[test]
fn kv_output_clone_and_debug() {
    let out = KvOutput {
        operation: KvOp::Delete,
        key: "temp:99".to_string(),
        metadata: vec![],
    };
    let cloned = out.clone();
    assert_eq!(cloned.operation, KvOp::Delete);
    assert_eq!(cloned.key, "temp:99");
    let dbg = format!("{:?}", out);
    assert!(dbg.contains("Delete"));
    assert!(dbg.contains("temp:99"));
}

// ======================================================================
// 17. OffsetLimit
// ======================================================================

#[test]
fn offset_limit_param() {
    let ol = OffsetLimit::Param;
    assert!(matches!(ol, OffsetLimit::Param));
}

#[test]
fn offset_limit_value() {
    let ol = OffsetLimit::Value(42);
    assert!(matches!(ol, OffsetLimit::Value(42)));
}

#[test]
fn offset_limit_value_zero() {
    let ol = OffsetLimit::Value(0);
    assert!(matches!(ol, OffsetLimit::Value(0)));
}

#[test]
fn offset_limit_clone() {
    let ol = OffsetLimit::Value(10);
    let cloned = ol.clone();
    assert!(matches!(cloned, OffsetLimit::Value(10)));
}

// ======================================================================
// 18. TransactionIR
// ======================================================================

#[test]
fn transaction_ir_begin() {
    assert!(matches!(TransactionIR::Begin, TransactionIR::Begin));
}

#[test]
fn transaction_ir_commit() {
    assert!(matches!(TransactionIR::Commit, TransactionIR::Commit));
}

#[test]
fn transaction_ir_rollback() {
    assert!(matches!(TransactionIR::Rollback, TransactionIR::Rollback));
}

#[test]
fn transaction_ir_savepoint() {
    let sp = TransactionIR::Savepoint("sp1".to_string());
    assert!(matches!(sp, TransactionIR::Savepoint(_)));
    if let TransactionIR::Savepoint(name) = sp {
        assert_eq!(name, "sp1");
    }
}

#[test]
fn transaction_ir_release_savepoint() {
    let sp = TransactionIR::ReleaseSavepoint("sp1".to_string());
    assert!(matches!(sp, TransactionIR::ReleaseSavepoint(_)));
}

#[test]
fn transaction_ir_rollback_to_savepoint() {
    let sp = TransactionIR::RollbackToSavepoint("sp2".to_string());
    assert!(matches!(sp, TransactionIR::RollbackToSavepoint(_)));
}

#[test]
fn transaction_ir_block() {
    let block = TransactionIR::Block(vec![
        Statement::Transaction(TransactionIR::Begin),
        Statement::Transaction(TransactionIR::Commit),
    ]);
    if let TransactionIR::Block(stmts) = block {
        assert_eq!(stmts.len(), 2);
    } else {
        panic!("expected Block");
    }
}

// ======================================================================
// 19. AlterAction
// ======================================================================

#[test]
fn alter_action_add_field() {
    let action = AlterAction::AddField(FieldDef::new("email", DataType::Text));
    assert!(matches!(action, AlterAction::AddField(_)));
}

#[test]
fn alter_action_drop_field() {
    let action = AlterAction::DropField("old_col".to_string());
    assert!(matches!(action, AlterAction::DropField(_)));
}

#[test]
fn alter_action_rename_field() {
    let action = AlterAction::RenameField {
        from: "old".to_string(),
        to: "new".to_string(),
    };
    assert!(matches!(action, AlterAction::RenameField { .. }));
}

#[test]
fn alter_action_alter_field_type() {
    let action = AlterAction::AlterFieldType {
        name: "status".to_string(),
        new_type: DataType::Int32,
    };
    assert!(matches!(action, AlterAction::AlterFieldType { .. }));
}

#[test]
fn alter_action_set_field_default() {
    let action = AlterAction::SetFieldDefault {
        name: "status".to_string(),
        expr: "'active'".to_string(),
    };
    assert!(matches!(action, AlterAction::SetFieldDefault { .. }));
}

#[test]
fn alter_action_drop_field_default() {
    let action = AlterAction::DropFieldDefault("status".to_string());
    assert!(matches!(action, AlterAction::DropFieldDefault(_)));
}

#[test]
fn alter_action_set_field_not_null() {
    let action = AlterAction::SetFieldNotNull("email".to_string());
    assert!(matches!(action, AlterAction::SetFieldNotNull(_)));
}

#[test]
fn alter_action_drop_field_not_null() {
    let action = AlterAction::DropFieldNotNull("bio".to_string());
    assert!(matches!(action, AlterAction::DropFieldNotNull(_)));
}

#[test]
fn alter_action_add_constraint() {
    let action = AlterAction::AddConstraint(crate::constraint::EntityConstraint::Check("age > 0").into());
    assert!(matches!(action, AlterAction::AddConstraint(_)));
}

#[test]
fn alter_action_drop_constraint() {
    let action = AlterAction::DropConstraint("uq_email".to_string());
    assert!(matches!(action, AlterAction::DropConstraint(_)));
}

#[test]
fn alter_action_rename_model() {
    let action = AlterAction::RenameEntity("new_name".to_string());
    assert!(matches!(action, AlterAction::RenameEntity(_)));
}

// ======================================================================
// Additional coverage: StorageOutput, remaining Statement variants,
// compound queries, and IR struct construction
// ======================================================================

#[test]
fn storage_output_construction() {
    let out = StorageOutput {
        operation: StorageOp::PutObject {
            bucket: "media".to_string(),
            key: "avatar.jpg".to_string(),
            content_type: Some("image/jpeg".to_string()),
        },
    };
    assert!(matches!(out.operation, StorageOp::PutObject { .. }));
}

#[test]
fn statement_alter_model() {
    let stmt = Statement::AlterEntity(AlterEntityIR {
        target: entity_ref("users"),
        actions: vec![AlterAction::AddField(FieldDef::new("age", DataType::Int32))],
    });
    assert!(matches!(stmt, Statement::AlterEntity(_)));
}

#[test]
fn statement_define_index() {
    let stmt = Statement::DefineIndex(DefineIndexIR {
        name: "idx_email".to_string(),
        target: entity_ref("users"),
        columns: vec!["email".to_string()],
        unique: true,
        if_not_exists: false,
        concurrently: false,
        method: Some(IndexMethod::BTree),
        where_clause: None,
    });
    assert!(matches!(stmt, Statement::DefineIndex(_)));
}

#[test]
fn statement_drop_index() {
    let stmt = Statement::DropIndex(DropIndexIR {
        name: "idx_email".to_string(),
        if_exists: true,
        concurrently: false,
        cascade: false,
    });
    assert!(matches!(stmt, Statement::DropIndex(_)));
}

#[test]
fn statement_define_type() {
    let stmt = Statement::DefineType(DefineTypeIR {
        name: "mood".to_string(),
        namespace: None,
        variants: vec!["happy".to_string(), "sad".to_string()],
    });
    assert!(matches!(stmt, Statement::DefineType(_)));
}

#[test]
fn statement_drop_type() {
    let stmt = Statement::DropType(DropTypeIR {
        name: "mood".to_string(),
        if_exists: false,
    });
    assert!(matches!(stmt, Statement::DropType(_)));
}

#[test]
fn statement_insert_select() {
    let stmt = Statement::InsertSelect(InsertSelectIR {
        target: entity_ref("archive"),
        fields: vec!["id".to_string(), "name".to_string()],
        source_query: "SELECT id, name FROM users WHERE archived".to_string(),
        returning: vec![],
    });
    assert!(matches!(stmt, Statement::InsertSelect(_)));
}

#[test]
fn statement_update() {
    let stmt = Statement::Update(UpdateIR {
        target: entity_ref("users"),
        assignments: vec![],
        filters: vec![],
        returning: vec![],
    });
    assert!(matches!(stmt, Statement::Update(_)));
}

#[test]
fn statement_remove() {
    let stmt = Statement::Remove(RemoveIR {
        target: entity_ref("users"),
        filters: vec![],
        returning: vec![],
    });
    assert!(matches!(stmt, Statement::Remove(_)));
}

#[test]
fn statement_upsert() {
    let stmt = Statement::Upsert(Box::new(UpsertIR {
        target: entity_ref("users"),
        fields: vec!["email".to_string()],
        conflict_fields: vec!["email".to_string()],
        conflict_constraint: None,
        update_fields: vec!["name".to_string()],
        do_nothing: false,
        conflict_filters: vec![],
        returning: vec![],
    }));
    assert!(matches!(stmt, Statement::Upsert(_)));
}

#[test]
fn statement_compound() {
    let stmt = Statement::Compound(Box::new(CompoundQueryIR {
        base: Box::new(simple_query_ir()),
        operations: vec![(SetOpKind::Union, simple_query_ir())],
        order_by: vec![],
        offset: None,
        limit: None,
    }));
    assert!(matches!(stmt, Statement::Compound(_)));
}

#[test]
fn statement_revoke() {
    let stmt = Statement::Revoke(RevokeIR {
        privilege: Privilege::All,
        on_target: "users".to_string(),
        from_role: "guest".to_string(),
    });
    assert!(matches!(stmt, Statement::Revoke(_)));
}

#[test]
fn statement_define_policy() {
    let stmt = Statement::DefinePolicy(DefinePolicyIR {
        name: "tenant_isolation".to_string(),
        on_model: "users".to_string(),
        action: PolicyAction::All,
        using_expr: None,
        check_expr: None,
    });
    assert!(matches!(stmt, Statement::DefinePolicy(_)));
}

#[test]
fn statement_get_object() {
    let stmt = Statement::GetObject(GetObjectIR {
        key: "doc.pdf".to_string(),
        bucket: "docs".to_string(),
    });
    assert!(matches!(stmt, Statement::GetObject(_)));
}

#[test]
fn statement_list_objects() {
    let stmt = Statement::ListObjects(ListObjectsIR {
        bucket: "uploads".to_string(),
        prefix: Some("img/".to_string()),
        limit: Some(100),
        continuation_token: None,
    });
    assert!(matches!(stmt, Statement::ListObjects(_)));
}

#[test]
fn statement_write_file() {
    let stmt = Statement::WriteFile(WriteFileIR {
        path: "/out/report.csv".to_string(),
        source: ObjectSource::FromBytes,
        create_dirs: true,
    });
    assert!(matches!(stmt, Statement::WriteFile(_)));
}

#[test]
fn statement_move_file() {
    let stmt = Statement::MoveFile(MoveFileIR {
        from: "/a/old.txt".to_string(),
        to: "/b/new.txt".to_string(),
    });
    assert!(matches!(stmt, Statement::MoveFile(_)));
}

#[test]
fn object_source_from_path() {
    let src = ObjectSource::FromPath("/data/file.bin".to_string());
    assert!(matches!(src, ObjectSource::FromPath(_)));
}

#[test]
fn object_source_from_bytes() {
    let src = ObjectSource::FromBytes;
    assert!(matches!(src, ObjectSource::FromBytes));
}

#[test]
fn object_source_from_expr() {
    let src = ObjectSource::FromExpr(crate::expr::Expr::Identifier("col".to_string()));
    assert!(matches!(src, ObjectSource::FromExpr(_)));
}

#[test]
fn join_ir_construction() {
    let join = JoinIR {
        join_type: JoinType::Left,
        target: entity_ref("orders"),
        on_conditions: vec![("users.id".to_string(), "orders.user_id".to_string())],
    };
    assert_eq!(join.join_type, JoinType::Left);
    assert_eq!(join.target.name, "orders");
    assert_eq!(join.on_conditions.len(), 1);
}

#[test]
fn query_ir_with_options() {
    let q = QueryIR {
        source: entity_ref("users"),
        projections: vec![crate::expr::Expr::Identifier("id".to_string())],
        joins: vec![],
        filters: vec![],
        group_by: vec![],
        having: vec![],
        order_by: vec![],
        offset: Some(OffsetLimit::Value(10)),
        limit: Some(OffsetLimit::Value(20)),
        distinct: true,
        distinct_on: vec!["email".to_string()],
        lock_mode: Some(LockMode::ForUpdate),
    };
    assert!(q.distinct);
    assert!(matches!(q.offset, Some(OffsetLimit::Value(10))));
    assert!(matches!(q.limit, Some(OffsetLimit::Value(20))));
    assert_eq!(q.lock_mode, Some(LockMode::ForUpdate));
}

#[test]
fn define_model_ir_with_fields() {
    let ir = DefineEntityIR {
        name: "products".to_string(),
        namespace: Some("shop".to_string()),
        fields: vec![
            FieldDef::new("id", DataType::Uuid).primary_key(),
            FieldDef::new("name", DataType::Text),
            FieldDef::new("price", DataType::Decimal { precision: None, scale: None }),
        ],
        constraints: vec![],
        if_not_exists: true,
    };
    assert_eq!(ir.name, "products");
    assert_eq!(ir.namespace.as_deref(), Some("shop"));
    assert_eq!(ir.fields.len(), 3);
    assert!(ir.if_not_exists);
}

#[test]
fn define_index_ir_concurrently_with_where() {
    let ir = DefineIndexIR {
        name: "idx_active_users".to_string(),
        target: entity_ref("users"),
        columns: vec!["email".to_string()],
        unique: false,
        if_not_exists: true,
        concurrently: true,
        method: Some(IndexMethod::Gin),
        where_clause: Some("active = true".to_string()),
    };
    assert!(ir.concurrently);
    assert_eq!(ir.method, Some(IndexMethod::Gin));
    assert_eq!(ir.where_clause.as_deref(), Some("active = true"));
}
