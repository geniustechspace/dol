use super::*;
use dol_entity::{Field, DataType};
use dol_core::expr::{Expr, field, param};

fn users_entity() -> Entity {
    Entity::new(
        "users",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("email", DataType::Text).unique(),
            Field::new("name", DataType::Text),
        ],
    )
}

// ── Query construction ──────────────────────────────────────────

#[test]
fn from_entity() {
    let users = users_entity();
    let q = Query::from(&users);
    assert_eq!(q.name, "users");
    assert!(q.namespace.is_none());
    assert_eq!(
        q.field_names.as_deref(),
        Some(["id", "email", "name"].map(String::from).as_slice())
    );
}

#[test]
fn from_plain_string() {
    let q = Query::from("users");
    assert_eq!(q.name, "users");
    assert!(q.namespace.is_none());
    assert!(q.field_names.is_none());
}

#[test]
fn from_namespaced_string() {
    let q = Query::from("identity.users");
    assert_eq!(q.name, "users");
    assert_eq!(q.namespace.as_deref(), Some("identity"));
    assert!(q.field_names.is_none());
}

#[test]
fn from_owned_string() {
    let q = Query::from(String::from("identity.users"));
    assert_eq!(q.name, "users");
    assert_eq!(q.namespace.as_deref(), Some("identity"));
}

// ── Namespace chaining ──────────────────────────────────────────

#[test]
fn namespace_single_segment() {
    let q = Query::from("api").namespace("users");
    assert_eq!(q.name, "users");
    assert_eq!(q.namespace.as_deref(), Some("api"));
}

#[test]
fn namespace_chained_segments() {
    let q = Query::from("api").namespace("v1").namespace("users");
    assert_eq!(q.name, "users");
    assert_eq!(q.namespace.as_deref(), Some("api.v1"));
}

#[test]
fn namespace_three_levels() {
    let q = Query::from("api")
        .namespace("v1")
        .namespace("admin")
        .namespace("users");
    assert_eq!(q.name, "users");
    assert_eq!(q.namespace.as_deref(), Some("api.v1.admin"));
}

#[test]
fn namespace_on_already_namespaced_string() {
    // "identity.users" → namespace="identity", name="users"
    // .namespace("profiles") → namespace="identity.users", name="profiles"
    let q = Query::from("identity.users").namespace("profiles");
    assert_eq!(q.name, "profiles");
    assert_eq!(q.namespace.as_deref(), Some("identity.users"));
}

#[test]
fn namespace_propagates_to_get_ir() {
    let ir = Query::from("api")
        .namespace("v1")
        .namespace("users")
        .get()
        .fields(&["id"])
        .build();
    assert_eq!(ir.source.name, "users");
    assert_eq!(ir.source.namespace.as_deref(), Some("api.v1"));
}

#[test]
fn namespace_propagates_to_insert_ir() {
    let ir = Query::from("api")
        .namespace("users")
        .insert()
        .fields(&["id", "email"])
        .build();
    assert_eq!(ir.target.name, "users");
    assert_eq!(ir.target.namespace.as_deref(), Some("api"));
}

#[test]
fn namespace_propagates_to_update_ir() {
    let ir = Query::from("api")
        .namespace("users")
        .update()
        .set("email")
        .build();
    assert_eq!(ir.target.name, "users");
    assert_eq!(ir.target.namespace.as_deref(), Some("api"));
}

#[test]
fn namespace_propagates_to_remove_ir() {
    let ir = Query::from("api")
        .namespace("users")
        .remove()
        .filter(field("id").eq(param()))
        .build();
    assert_eq!(ir.target.name, "users");
    assert_eq!(ir.target.namespace.as_deref(), Some("api"));
}

#[test]
fn namespace_propagates_to_upsert_ir() {
    let ir = Query::from("api")
        .namespace("users")
        .upsert()
        .fields(&["id", "email"])
        .on_conflict(&["id"])
        .do_nothing()
        .build();
    assert_eq!(ir.target.name, "users");
    assert_eq!(ir.target.namespace.as_deref(), Some("api"));
}

// ── GetQuery from Entity ────────────────────────────────────────

#[test]
fn get_from_entity_defaults() {
    let users = users_entity();
    let ir = Query::from(&users).get().build();
    assert_eq!(ir.source.name, "users");
    assert_eq!(ir.projections.len(), 3);
    assert!(matches!(&ir.projections[0], Expr::Ref(p) if p.as_single() == Some("id")));
    assert!(matches!(&ir.projections[2], Expr::Ref(p) if p.as_single() == Some("name")));
}

#[test]
fn get_from_entity_filter() {
    let users = users_entity();
    let ir = Query::from(&users)
        .get()
        .filter(field("id").eq(param()))
        .build();
    assert_eq!(ir.filters.len(), 1);
}

// ── GetQuery from string ────────────────────────────────────────

#[test]
fn get_from_string_columns() {
    let ir = Query::from("users")
        .get()
        .fields(&["id", "email"])
        .filter(field("id").eq(param()))
        .build();
    assert_eq!(ir.source.name, "users");
    assert!(ir.source.namespace.is_none());
    assert_eq!(ir.projections.len(), 2);
    assert_eq!(ir.filters.len(), 1);
}

#[test]
fn get_from_namespaced_string() {
    let ir = Query::from("identity.users").get().fields(&["id"]).build();
    assert_eq!(ir.source.name, "users");
    assert_eq!(ir.source.namespace.as_deref(), Some("identity"));
}

// ── InsertQuery ─────────────────────────────────────────────────

#[test]
fn insert_from_entity_defaults() {
    let users = users_entity();
    let ir = Query::from(&users).insert().build();
    assert_eq!(ir.target.name, "users");
    assert_eq!(ir.fields, ["id", "email", "name"]);
}

#[test]
fn insert_from_string_columns() {
    let ir = Query::from("users")
        .insert()
        .fields(&["id", "email"])
        .rows(2)
        .returning_all()
        .build();
    assert_eq!(ir.target.name, "users");
    assert_eq!(ir.fields, ["id", "email"]);
    assert_eq!(ir.row_count, 2);
    assert_eq!(ir.returning, ["*"]);
}

// ── UpdateQuery ─────────────────────────────────────────────────

#[test]
fn update_from_string() {
    let ir = Query::from("users")
        .update()
        .set("email")
        .filter(field("id").eq(param()))
        .returning_all()
        .build();
    assert_eq!(ir.target.name, "users");
    assert_eq!(ir.assignments.len(), 1);
    assert_eq!(ir.filters.len(), 1);
    assert_eq!(ir.returning, ["*"]);
}

// ── RemoveQuery ─────────────────────────────────────────────────

#[test]
fn remove_from_string() {
    let ir = Query::from("users")
        .remove()
        .filter(field("id").eq(param()))
        .returning_all()
        .build();
    assert_eq!(ir.target.name, "users");
    assert_eq!(ir.filters.len(), 1);
    assert_eq!(ir.returning, ["*"]);
}

// ── UpsertQuery ─────────────────────────────────────────────────

#[test]
fn upsert_from_string() {
    let ir = Query::from("users")
        .upsert()
        .fields(&["id", "email", "name"])
        .on_conflict(&["id"])
        .do_update(&["email", "name"])
        .build();
    assert_eq!(ir.target.name, "users");
    assert_eq!(ir.fields, ["id", "email", "name"]);
    assert_eq!(ir.conflict_fields, ["id"]);
    assert_eq!(ir.update_fields, ["email", "name"]);
}

#[test]
fn upsert_do_nothing() {
    let ir = Query::from("users")
        .upsert()
        .fields(&["id", "email"])
        .on_conflict(&["id"])
        .do_nothing()
        .build();
    assert!(ir.do_nothing);
    assert!(ir.update_fields.is_empty());
}

// ===========================================================================
// build_ir() tests — verify arena-based IR construction
// ===========================================================================

#[test]
fn get_build_ir_produces_query_statement() {
    let (stmt, _arena, interner) = Query::from("users")
        .get()
        .fields(&["id", "email"])
        .filter(field("id").eq(param()))
        .build_ir();
    match stmt {
        dol_ir::Statement::Query(q) => {
            assert_eq!(interner.get(q.from), "users");
            assert_eq!(q.columns.len(), 2);
            assert_ne!(q.filter, dol_expr::NULL_NODE);
        }
        _ => panic!("expected Statement::Query"),
    }
}

#[test]
fn get_build_ir_with_namespace() {
    let (stmt, _arena, interner) = Query::from("identity.users")
        .get()
        .fields(&["id"])
        .build_ir();
    match stmt {
        dol_ir::Statement::Query(q) => {
            assert_eq!(interner.get(q.from), "identity.users");
            assert_eq!(q.columns.len(), 1);
        }
        _ => panic!("expected Statement::Query"),
    }
}

#[test]
fn insert_build_ir_produces_insert_statement() {
    let (stmt, _arena, interner) = Query::from("users")
        .insert()
        .fields(&["id", "email"])
        .rows(2)
        .build_ir();
    match stmt {
        dol_ir::Statement::Insert(ins) => {
            assert_eq!(interner.get(ins.target), "users");
            assert_eq!(ins.columns.len(), 2);
            // 2 rows × 2 columns = 4 param nodes
            assert_eq!(ins.values.len(), 4);
        }
        _ => panic!("expected Statement::Insert"),
    }
}

#[test]
fn insert_build_ir_returning() {
    let (stmt, _arena, interner) = Query::from("users")
        .insert()
        .fields(&["id"])
        .returning_all()
        .build_ir();
    match stmt {
        dol_ir::Statement::Insert(ins) => {
            assert_eq!(ins.returning.len(), 1);
            // The "*" returning column
            let ret_col = interner.get(ins.columns[0]);
            assert_eq!(ret_col, "id");
        }
        _ => panic!("expected Statement::Insert"),
    }
}

#[test]
fn update_build_ir_produces_update_statement() {
    let (stmt, _arena, interner) = Query::from("users")
        .update()
        .set("email")
        .filter(field("id").eq(param()))
        .build_ir();
    match stmt {
        dol_ir::Statement::Update(upd) => {
            assert_eq!(interner.get(upd.target), "users");
            assert_eq!(upd.columns.len(), 1);
            assert_eq!(upd.values.len(), 1);
            assert_ne!(upd.filter, dol_expr::NULL_NODE);
        }
        _ => panic!("expected Statement::Update"),
    }
}

#[test]
fn remove_build_ir_produces_delete_statement() {
    let (stmt, _arena, interner) = Query::from("users")
        .remove()
        .filter(field("id").eq(param()))
        .build_ir();
    match stmt {
        dol_ir::Statement::Delete(del) => {
            assert_eq!(interner.get(del.target), "users");
            assert_ne!(del.filter, dol_expr::NULL_NODE);
        }
        _ => panic!("expected Statement::Delete"),
    }
}

#[test]
fn upsert_build_ir_do_nothing() {
    let (stmt, _arena, interner) = Query::from("users")
        .upsert()
        .fields(&["id", "email"])
        .on_conflict(&["id"])
        .do_nothing()
        .build_ir();
    match stmt {
        dol_ir::Statement::Upsert(ups) => {
            assert_eq!(interner.get(ups.target), "users");
            assert_eq!(ups.columns.len(), 2);
            assert!(matches!(ups.conflict, Some(dol_expr::expr::ConflictClause::DoNothing)));
        }
        _ => panic!("expected Statement::Upsert"),
    }
}

#[test]
fn upsert_build_ir_do_update() {
    let (stmt, _arena, _interner) = Query::from("users")
        .upsert()
        .fields(&["id", "email", "name"])
        .on_conflict(&["id"])
        .do_update(&["email", "name"])
        .build_ir();
    match stmt {
        dol_ir::Statement::Upsert(ups) => {
            assert_eq!(ups.columns.len(), 3);
            match ups.conflict {
                Some(dol_expr::expr::ConflictClause::DoUpdate { assignments }) => {
                    assert_eq!(assignments.len(), 2);
                }
                _ => panic!("expected DoUpdate conflict"),
            }
        }
        _ => panic!("expected Statement::Upsert"),
    }
}

#[test]
fn get_build_ir_default_entity_fields() {
    let users = users_entity();
    let (stmt, _arena, interner) = Query::from(&users)
        .get()
        .build_ir();
    match stmt {
        dol_ir::Statement::Query(q) => {
            assert_eq!(interner.get(q.from), "users");
            // Should select all 3 entity fields by default.
            assert_eq!(q.columns.len(), 3);
        }
        _ => panic!("expected Statement::Query"),
    }
}
