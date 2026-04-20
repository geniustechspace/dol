use super::*;
use dol_core::expr::{field, int, param, string};
use dol_core::ir::definition::{DefineEntityIR, DropEntityIR, FieldDef, OwnedForeignKeyRef};
use dol_core::ir::mutation::{InsertIR, RemoveIR, UpdateIR};
use dol_core::ir::query::{JoinIR, JoinType, LockMode, QueryIR};
use dol_core::ir::{AlterAction, AlterEntityIR, EntityRef, OffsetLimit};
use dol_core::types::DataType;

fn entity_ref(name: &str) -> EntityRef {
    EntityRef {
        name: name.to_string(),
        namespace: None,
        alias: None,
    }
}

fn entity_ref_ns(ns: &str, name: &str) -> EntityRef {
    EntityRef {
        name: name.to_string(),
        namespace: Some(ns.to_string()),
        alias: None,
    }
}

// ── DefineEntity → CreateSheet ─────────────────────────────────────────

#[test]
fn define_entity_creates_sheet() {
    let ir = DefineEntityIR {
        name: "users".to_string(),
        namespace: None,
        fields: vec![
            FieldDef::new("id", DataType::Uuid),
            FieldDef::new("email", DataType::Text),
            FieldDef::new("age", DataType::Int32),
        ],
        constraints: vec![],
        if_not_exists: false,
    };

    let stmt = Statement::DefineEntity(Box::new(ir));
    let result = SpreadsheetBackend.render(&stmt).unwrap();

    {
        let out = result;
        assert_eq!(out.sheet, "users");
        assert!(out.workbook.is_none());
        match out.operation {
            SpreadsheetOp::CreateSheet {
                columns,
                if_not_exists,
            } => {
                assert_eq!(columns.len(), 3);
                assert_eq!(columns[0].name, "id");
                assert_eq!(columns[0].data_type, DataType::Uuid);
                assert_eq!(columns[1].name, "email");
                assert_eq!(columns[2].name, "age");
                assert!(!if_not_exists);
            }
            _ => panic!("expected CreateSheet"),
        }
    }
}

#[test]
fn define_entity_with_namespace() {
    let ir = DefineEntityIR {
        name: "users".to_string(),
        namespace: Some("hr".to_string()),
        fields: vec![FieldDef::new("id", DataType::Uuid)],
        constraints: vec![],
        if_not_exists: true,
    };

    let stmt = Statement::DefineEntity(Box::new(ir));
    let result = SpreadsheetBackend.render(&stmt).unwrap();

    {
        let out = result;
        assert_eq!(out.sheet, "hr.users");
        match out.operation {
            SpreadsheetOp::CreateSheet { if_not_exists, .. } => {
                assert!(if_not_exists);
            }
            _ => panic!("expected CreateSheet"),
        }
    }
}

#[test]
fn define_entity_rejects_primary_key() {
    let ir = DefineEntityIR {
        name: "users".to_string(),
        namespace: None,
        fields: vec![FieldDef::new("id", DataType::Uuid).primary_key()],
        constraints: vec![],
        if_not_exists: false,
    };
    let stmt = Statement::DefineEntity(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn define_entity_rejects_unique() {
    let ir = DefineEntityIR {
        name: "users".to_string(),
        namespace: None,
        fields: vec![FieldDef::new("email", DataType::Text).unique()],
        constraints: vec![],
        if_not_exists: false,
    };
    let stmt = Statement::DefineEntity(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn define_entity_rejects_references() {
    let ir = DefineEntityIR {
        name: "orders".to_string(),
        namespace: None,
        fields: vec![
            FieldDef::new("user_id", DataType::Uuid)
                .references(OwnedForeignKeyRef::new("users", "id")),
        ],
        constraints: vec![],
        if_not_exists: false,
    };
    let stmt = Statement::DefineEntity(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn define_entity_rejects_check() {
    let ir = DefineEntityIR {
        name: "items".to_string(),
        namespace: None,
        fields: vec![FieldDef::new("qty", DataType::Int32).check("qty > 0")],
        constraints: vec![],
        if_not_exists: false,
    };
    let stmt = Statement::DefineEntity(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn define_entity_rejects_generated() {
    let ir = DefineEntityIR {
        name: "items".to_string(),
        namespace: None,
        fields: vec![FieldDef::new("total", DataType::Int32).generated_stored("qty * price")],
        constraints: vec![],
        if_not_exists: false,
    };
    let stmt = Statement::DefineEntity(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn define_entity_rejects_auto_increment() {
    let ir = DefineEntityIR {
        name: "items".to_string(),
        namespace: None,
        fields: vec![FieldDef::new("id", DataType::Int64).auto_increment()],
        constraints: vec![],
        if_not_exists: false,
    };
    let stmt = Statement::DefineEntity(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn define_entity_rejects_table_constraints() {
    use dol_core::ir::definition::OwnedEntityConstraint;
    let ir = DefineEntityIR {
        name: "users".to_string(),
        namespace: None,
        fields: vec![FieldDef::new("id", DataType::Uuid)],
        constraints: vec![OwnedEntityConstraint::Unique(vec!["id".to_string()])],
        if_not_exists: false,
    };
    let stmt = Statement::DefineEntity(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

// ── DropEntity → DropSheet ─────────────────────────────────────────────

#[test]
fn drop_entity_drops_sheet() {
    let ir = DropEntityIR {
        target: entity_ref("users"),
        if_exists: true,
        cascade: false,
    };

    let stmt = Statement::DropEntity(ir);
    let result = SpreadsheetBackend.render(&stmt).unwrap();

    {
        let out = result;
        assert_eq!(out.sheet, "users");
        match out.operation {
            SpreadsheetOp::DropSheet { if_exists } => {
                assert!(if_exists);
            }
            _ => panic!("expected DropSheet"),
        }
    }
}

#[test]
fn drop_entity_rejects_cascade() {
    let ir = DropEntityIR {
        target: entity_ref("users"),
        if_exists: false,
        cascade: true,
    };
    let stmt = Statement::DropEntity(ir);
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

// ── AlterEntity (rename) → RenameSheet ─────────────────────────────────

#[test]
fn alter_entity_rename_to_rename_sheet() {
    let ir = AlterEntityIR {
        target: entity_ref("users"),
        actions: vec![AlterAction::RenameEntity("employees".to_string())],
    };

    let stmt = Statement::AlterEntity(ir);
    let result = SpreadsheetBackend.render(&stmt).unwrap();

    {
        let out = result;
        assert_eq!(out.sheet, "users");
        match out.operation {
            SpreadsheetOp::RenameSheet { new_name } => {
                assert_eq!(new_name, "employees");
            }
            _ => panic!("expected RenameSheet"),
        }
    }
}

#[test]
fn alter_entity_add_field_unsupported() {
    let ir = AlterEntityIR {
        target: entity_ref("users"),
        actions: vec![AlterAction::AddField(FieldDef::new(
            "phone",
            DataType::Text,
        ))],
    };

    let stmt = Statement::AlterEntity(ir);
    let result = SpreadsheetBackend.render(&stmt);
    assert!(result.is_err());
}

#[test]
fn alter_entity_empty_actions_unsupported() {
    let ir = AlterEntityIR {
        target: entity_ref("users"),
        actions: vec![],
    };
    let stmt = Statement::AlterEntity(ir);
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn alter_entity_multiple_actions_unsupported() {
    let ir = AlterEntityIR {
        target: entity_ref("users"),
        actions: vec![
            AlterAction::RenameEntity("employees".to_string()),
            AlterAction::AddField(FieldDef::new("phone", DataType::Text)),
        ],
    };
    let stmt = Statement::AlterEntity(ir);
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

// ── Insert → AppendRows ────────────────────────────────────────────────

#[test]
fn insert_appends_rows() {
    let ir = InsertIR {
        target: entity_ref("users"),
        fields: vec!["id".to_string(), "email".to_string()],
        row_count: 3,
        returning: vec![],
    };

    let stmt = Statement::Insert(ir);
    let result = SpreadsheetBackend.render(&stmt).unwrap();

    {
        let out = result;
        assert_eq!(out.sheet, "users");
        match out.operation {
            SpreadsheetOp::AppendRows { columns, row_count } => {
                assert_eq!(columns, vec!["id", "email"]);
                assert_eq!(row_count, 3);
            }
            _ => panic!("expected AppendRows"),
        }
    }
}

#[test]
fn insert_rejects_returning() {
    let ir = InsertIR {
        target: entity_ref("users"),
        fields: vec!["id".to_string()],
        row_count: 1,
        returning: vec!["id".to_string()],
    };
    let stmt = Statement::Insert(ir);
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

// ── Update → UpdateRows ────────────────────────────────────────────────

#[test]
fn update_with_filter() {
    let ir = UpdateIR {
        target: entity_ref("users"),
        assignments: vec![("email".to_string(), string("new@example.com"))],
        filters: vec![field("id").eq(param())],
        returning: vec![],
    };

    let stmt = Statement::Update(ir);
    let result = SpreadsheetBackend.render(&stmt).unwrap();

    {
        let out = result;
        assert_eq!(out.sheet, "users");
        match out.operation {
            SpreadsheetOp::UpdateRows {
                assignments,
                filter,
            } => {
                assert_eq!(assignments.len(), 1);
                assert_eq!(assignments[0].0, "email");
                assert!(filter.is_some());
                let f = filter.unwrap();
                assert!(f.contains("id"));
                assert!(f.contains("="));
            }
            _ => panic!("expected UpdateRows"),
        }
    }
}

#[test]
fn update_without_filter() {
    let ir = UpdateIR {
        target: entity_ref("users"),
        assignments: vec![("status".to_string(), string("archived"))],
        filters: vec![],
        returning: vec![],
    };

    let stmt = Statement::Update(ir);
    let result = SpreadsheetBackend.render(&stmt).unwrap();

    match result.operation {
        SpreadsheetOp::UpdateRows { filter, .. } => {
            assert!(filter.is_none());
        }
        _ => panic!("expected UpdateRows"),
    }
}

#[test]
fn update_rejects_returning() {
    let ir = UpdateIR {
        target: entity_ref("users"),
        assignments: vec![("email".to_string(), string("x@y.com"))],
        filters: vec![],
        returning: vec!["id".to_string()],
    };
    let stmt = Statement::Update(ir);
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

// ── Remove → DeleteRows ────────────────────────────────────────────────

#[test]
fn remove_with_filter() {
    let ir = RemoveIR {
        target: entity_ref("users"),
        filters: vec![field("status").eq(string("inactive"))],
        returning: vec![],
    };

    let stmt = Statement::Remove(ir);
    let result = SpreadsheetBackend.render(&stmt).unwrap();

    {
        let out = result;
        assert_eq!(out.sheet, "users");
        match out.operation {
            SpreadsheetOp::DeleteRows { filter } => {
                assert!(filter.is_some());
                let f = filter.unwrap();
                assert!(f.contains("status"));
            }
            _ => panic!("expected DeleteRows"),
        }
    }
}

#[test]
fn remove_without_filter() {
    let ir = RemoveIR {
        target: entity_ref("users"),
        filters: vec![],
        returning: vec![],
    };

    let stmt = Statement::Remove(ir);
    let result = SpreadsheetBackend.render(&stmt).unwrap();

    match result.operation {
        SpreadsheetOp::DeleteRows { filter } => {
            assert!(filter.is_none());
        }
        _ => panic!("expected DeleteRows"),
    }
}

#[test]
fn remove_rejects_returning() {
    let ir = RemoveIR {
        target: entity_ref("users"),
        filters: vec![],
        returning: vec!["id".to_string()],
    };
    let stmt = Statement::Remove(ir);
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

// ── Query → ReadRows ───────────────────────────────────────────────────

#[test]
fn query_read_rows_basic() {
    let ir = QueryIR {
        source: entity_ref("users"),
        projections: vec![field("id"), field("email")],
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
    };

    let stmt = Statement::Query(Box::new(ir));
    let result = SpreadsheetBackend.render(&stmt).unwrap();

    {
        let out = result;
        assert_eq!(out.sheet, "users");
        match out.operation {
            SpreadsheetOp::ReadRows {
                columns,
                filter,
                sort,
                limit,
                offset,
                distinct,
            } => {
                assert_eq!(columns, vec!["id", "email"]);
                assert!(filter.is_none());
                assert!(sort.is_empty());
                assert!(limit.is_none());
                assert!(offset.is_none());
                assert!(!distinct);
            }
            _ => panic!("expected ReadRows"),
        }
    }
}

#[test]
fn query_with_filter_and_limit() {
    let ir = QueryIR {
        source: entity_ref("users"),
        projections: vec![field("email")],
        joins: vec![],
        filters: vec![field("age").gt(int(18i32))],
        group_by: vec![],
        having: vec![],
        order_by: vec![],
        offset: None,
        limit: Some(OffsetLimit::Value(10)),
        distinct: false,
        distinct_on: vec![],
        lock_mode: None,
    };

    let stmt = Statement::Query(Box::new(ir));
    let result = SpreadsheetBackend.render(&stmt).unwrap();

    match result.operation {
        SpreadsheetOp::ReadRows {
            columns,
            filter,
            limit,
            ..
        } => {
            assert_eq!(columns, vec!["email"]);
            assert!(filter.is_some());
            let f = filter.unwrap();
            assert!(f.contains("age"));
            assert!(f.contains(">"));
            assert_eq!(limit, Some(10));
        }
        _ => panic!("expected ReadRows"),
    }
}

#[test]
fn query_with_sort() {
    let ir = QueryIR {
        source: entity_ref("users"),
        projections: vec![field("name"), field("age")],
        joins: vec![],
        filters: vec![],
        group_by: vec![],
        having: vec![],
        order_by: vec![
            OrderByExpr {
                expr: field("age"),
                direction: dol_core::expr::Direction::Desc,
                nulls: None,
            },
            OrderByExpr {
                expr: field("name"),
                direction: dol_core::expr::Direction::Asc,
                nulls: None,
            },
        ],
        offset: Some(OffsetLimit::Value(5)),
        limit: Some(OffsetLimit::Value(20)),
        distinct: true,
        distinct_on: vec![],
        lock_mode: None,
    };

    let stmt = Statement::Query(Box::new(ir));
    let result = SpreadsheetBackend.render(&stmt).unwrap();

    match result.operation {
        SpreadsheetOp::ReadRows {
            sort,
            limit,
            offset,
            distinct,
            ..
        } => {
            assert_eq!(sort.len(), 2);
            assert_eq!(sort[0].column, "age");
            assert_eq!(sort[0].direction, Direction::Desc);
            assert_eq!(sort[1].column, "name");
            assert_eq!(sort[1].direction, Direction::Asc);
            assert_eq!(limit, Some(20));
            assert_eq!(offset, Some(5));
            assert!(distinct);
        }
        _ => panic!("expected ReadRows"),
    }
}

#[test]
fn query_rejects_joins() {
    let ir = QueryIR {
        source: entity_ref("users"),
        projections: vec![field("id")],
        joins: vec![JoinIR {
            join_type: JoinType::Inner,
            target: entity_ref("orders"),
            on_conditions: vec![("users.id".to_string(), "orders.user_id".to_string())],
        }],
        filters: vec![],
        group_by: vec![],
        having: vec![],
        order_by: vec![],
        offset: None,
        limit: None,
        distinct: false,
        distinct_on: vec![],
        lock_mode: None,
    };
    let stmt = Statement::Query(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn query_rejects_group_by() {
    let ir = QueryIR {
        source: entity_ref("users"),
        projections: vec![field("status")],
        joins: vec![],
        filters: vec![],
        group_by: vec![field("status")],
        having: vec![],
        order_by: vec![],
        offset: None,
        limit: None,
        distinct: false,
        distinct_on: vec![],
        lock_mode: None,
    };
    let stmt = Statement::Query(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn query_rejects_having() {
    let ir = QueryIR {
        source: entity_ref("users"),
        projections: vec![field("status")],
        joins: vec![],
        filters: vec![],
        group_by: vec![],
        having: vec![field("count").gt(int(5i32))],
        order_by: vec![],
        offset: None,
        limit: None,
        distinct: false,
        distinct_on: vec![],
        lock_mode: None,
    };
    let stmt = Statement::Query(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn query_rejects_distinct_on() {
    let ir = QueryIR {
        source: entity_ref("users"),
        projections: vec![field("id")],
        joins: vec![],
        filters: vec![],
        group_by: vec![],
        having: vec![],
        order_by: vec![],
        offset: None,
        limit: None,
        distinct: false,
        distinct_on: vec!["id".to_string()],
        lock_mode: None,
    };
    let stmt = Statement::Query(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn query_rejects_lock_mode() {
    let ir = QueryIR {
        source: entity_ref("users"),
        projections: vec![field("id")],
        joins: vec![],
        filters: vec![],
        group_by: vec![],
        having: vec![],
        order_by: vec![],
        offset: None,
        limit: None,
        distinct: false,
        distinct_on: vec![],
        lock_mode: Some(LockMode::ForUpdate),
    };
    let stmt = Statement::Query(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn query_rejects_parameterized_limit() {
    let ir = QueryIR {
        source: entity_ref("users"),
        projections: vec![field("id")],
        joins: vec![],
        filters: vec![],
        group_by: vec![],
        having: vec![],
        order_by: vec![],
        offset: None,
        limit: Some(OffsetLimit::Param),
        distinct: false,
        distinct_on: vec![],
        lock_mode: None,
    };
    let stmt = Statement::Query(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn query_rejects_parameterized_offset() {
    let ir = QueryIR {
        source: entity_ref("users"),
        projections: vec![field("id")],
        joins: vec![],
        filters: vec![],
        group_by: vec![],
        having: vec![],
        order_by: vec![],
        offset: Some(OffsetLimit::Param),
        limit: None,
        distinct: false,
        distinct_on: vec![],
        lock_mode: None,
    };
    let stmt = Statement::Query(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

// ── Expression renderer rejects unsupported variants ───────────────────

#[test]
fn expr_rejects_subquery() {
    let ir = QueryIR {
        source: entity_ref("users"),
        projections: vec![Expr::Subquery("SELECT 1".into())],
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
    };
    let stmt = Statement::Query(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn expr_rejects_window_function() {
    let ir = QueryIR {
        source: entity_ref("users"),
        projections: vec![Expr::Window {
            func: Box::new(Expr::CountStar),
            partition_by: vec![],
            order_by: vec![],
            frame: None,
        }],
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
    };
    let stmt = Statement::Query(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

// ── Unsupported operations ─────────────────────────────────────────────

#[test]
fn transaction_unsupported() {
    let stmt = Statement::Transaction(dol_core::ir::TransactionIR::Begin);
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn index_unsupported() {
    let ir = dol_core::ir::DefineIndexIR {
        name: "idx_email".to_string(),
        target: entity_ref("users"),
        columns: vec!["email".to_string()],
        unique: false,
        if_not_exists: false,
        concurrently: false,
        method: None,
        where_clause: None,
    };
    let stmt = Statement::DefineIndex(ir);
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn storage_operations_unsupported() {
    use dol_core::ir::storage::GetObjectIR;

    let ir = GetObjectIR {
        key: "key".to_string(),
        bucket: "bucket".to_string(),
    };
    let stmt = Statement::GetObject(ir);
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

// ── Rename with namespace ──────────────────────────────────────────────

#[test]
fn alter_entity_rename_qualifies_new_name_with_namespace() {
    let ir = AlterEntityIR {
        target: entity_ref_ns("hr", "users"),
        actions: vec![AlterAction::RenameEntity("employees".to_string())],
    };

    let stmt = Statement::AlterEntity(ir);
    let result = SpreadsheetBackend.render(&stmt).unwrap();

    {
        let out = result;
        assert_eq!(out.sheet, "hr.users");
        match out.operation {
            SpreadsheetOp::RenameSheet { new_name } => {
                assert_eq!(new_name, "hr.employees");
            }
            _ => panic!("expected RenameSheet"),
        }
    }
}

// ── Sort rejects NULLS ordering ────────────────────────────────────────

#[test]
fn query_rejects_nulls_first_ordering() {
    let ir = QueryIR {
        source: entity_ref("users"),
        projections: vec![field("name")],
        joins: vec![],
        filters: vec![],
        group_by: vec![],
        having: vec![],
        order_by: vec![OrderByExpr {
            expr: field("name"),
            direction: dol_core::expr::Direction::Asc,
            nulls: Some(dol_core::expr::NullsPosition::First),
        }],
        offset: None,
        limit: None,
        distinct: false,
        distinct_on: vec![],
        lock_mode: None,
    };
    let stmt = Statement::Query(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

// ── Projection validation ──────────────────────────────────────────────

#[test]
fn query_rejects_non_column_projection() {
    let ir = QueryIR {
        source: entity_ref("users"),
        projections: vec![field("age").gt(int(18i32))],
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
    };
    let stmt = Statement::Query(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

#[test]
fn query_rejects_alias_projection() {
    let ir = QueryIR {
        source: entity_ref("users"),
        projections: vec![Expr::Alias {
            expr: Box::new(field("email")),
            alias: "user_email".to_string(),
        }],
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
    };
    let stmt = Statement::Query(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}

// ── Sort rejects non-column expression ─────────────────────────────────

#[test]
fn query_rejects_non_column_sort_expr() {
    let ir = QueryIR {
        source: entity_ref("users"),
        projections: vec![field("name")],
        joins: vec![],
        filters: vec![],
        group_by: vec![],
        having: vec![],
        order_by: vec![OrderByExpr {
            expr: Expr::Func {
                name: dol_core::expr::FuncDef::custom("UPPER"),
                args: vec![field("name")],
            },
            direction: dol_core::expr::Direction::Asc,
            nulls: None,
        }],
        offset: None,
        limit: None,
        distinct: false,
        distinct_on: vec![],
        lock_mode: None,
    };
    let stmt = Statement::Query(Box::new(ir));
    assert!(SpreadsheetBackend.render(&stmt).is_err());
}
