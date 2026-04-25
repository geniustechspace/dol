use super::*;
use dol_expr::tree::{field, param};
use dol_schema::{DataType, Field};

fn users_entity() -> Entity {
    Entity::new(
        "users",
        vec![
            Field::new("id", DataType::Uuid).identity(),
            Field::new("email", DataType::unbounded_string()).unique(),
            Field::new("name", DataType::unbounded_string()),
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
    let (stmt, _arena, interner) = Query::from("api")
        .namespace("v1")
        .namespace("users")
        .get()
        .fields(&["id"])
        .build();
    match stmt {
        dol_ir::Statement::Query(q) => {
            assert_eq!(interner.get(q.from), "api.v1.users");
        }
        _ => panic!("expected Statement::Query"),
    }
}

#[test]
fn namespace_propagates_to_insert_ir() {
    let (stmt, _arena, interner) = Query::from("api")
        .namespace("users")
        .insert()
        .fields(&["id", "email"])
        .build();
    match stmt {
        dol_ir::Statement::Insert(ins) => {
            assert_eq!(interner.get(ins.target), "api.users");
        }
        _ => panic!("expected Statement::Insert"),
    }
}

#[test]
fn namespace_propagates_to_update_ir() {
    let (stmt, _arena, interner) = Query::from("api")
        .namespace("users")
        .update()
        .set("email")
        .build();
    match stmt {
        dol_ir::Statement::Update(upd) => {
            assert_eq!(interner.get(upd.target), "api.users");
        }
        _ => panic!("expected Statement::Update"),
    }
}

#[test]
fn namespace_propagates_to_delete_ir() {
    let (stmt, _arena, interner) = Query::from("api")
        .namespace("users")
        .delete()
        .filter(field("id").eq(param()))
        .build();
    match stmt {
        dol_ir::Statement::Delete(del) => {
            assert_eq!(interner.get(del.target), "api.users");
        }
        _ => panic!("expected Statement::Delete"),
    }
}

#[test]
fn namespace_propagates_to_upsert_ir() {
    let (stmt, _arena, interner) = Query::from("api")
        .namespace("users")
        .upsert()
        .fields(&["id", "email"])
        .on_conflict(&["id"])
        .do_nothing()
        .build();
    match stmt {
        dol_ir::Statement::Upsert(ups) => {
            assert_eq!(interner.get(ups.target), "api.users");
        }
        _ => panic!("expected Statement::Upsert"),
    }
}

// ── GetQuery from Entity ────────────────────────────────────────

#[test]
fn get_from_entity_defaults() {
    let users = users_entity();
    let (stmt, _arena, interner) = Query::from(&users).get().build();
    match stmt {
        dol_ir::Statement::Query(q) => {
            assert_eq!(interner.get(q.from), "users");
            // Should select all 3 entity fields by default.
            assert_eq!(q.columns.len(), 3);
        }
        _ => panic!("expected Statement::Query"),
    }
}

#[test]
fn get_from_entity_filter() {
    let users = users_entity();
    let (stmt, _arena, _interner) = Query::from(&users)
        .get()
        .filter(field("id").eq(param()))
        .build();
    match stmt {
        dol_ir::Statement::Query(q) => {
            assert_ne!(q.filter, dol_expr::NULL_NODE);
        }
        _ => panic!("expected Statement::Query"),
    }
}

// ── GetQuery from string ────────────────────────────────────────

#[test]
fn get_from_string_columns() {
    let (stmt, _arena, interner) = Query::from("users")
        .get()
        .fields(&["id", "email"])
        .filter(field("id").eq(param()))
        .build();
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
fn get_from_namespaced_string() {
    let (stmt, _arena, interner) = Query::from("identity.users").get().fields(&["id"]).build();
    match stmt {
        dol_ir::Statement::Query(q) => {
            assert_eq!(interner.get(q.from), "identity.users");
        }
        _ => panic!("expected Statement::Query"),
    }
}

// ── InsertQuery ─────────────────────────────────────────────────

#[test]
fn insert_from_entity_defaults() {
    let users = users_entity();
    let (stmt, _arena, interner) = Query::from(&users).insert().build();
    match stmt {
        dol_ir::Statement::Insert(ins) => {
            assert_eq!(interner.get(ins.target), "users");
            assert_eq!(ins.columns.len(), 3);
        }
        _ => panic!("expected Statement::Insert"),
    }
}

#[test]
fn insert_from_string_columns() {
    let (stmt, _arena, interner) = Query::from("users")
        .insert()
        .fields(&["id", "email"])
        .rows(2)
        .returning_all()
        .build();
    match stmt {
        dol_ir::Statement::Insert(ins) => {
            assert_eq!(interner.get(ins.target), "users");
            assert_eq!(ins.columns.len(), 2);
            // 2 rows × 2 columns = 4 param nodes
            assert_eq!(ins.values.len(), 4);
            assert_eq!(ins.returning.len(), 1);
        }
        _ => panic!("expected Statement::Insert"),
    }
}

// ── UpdateQuery ─────────────────────────────────────────────────

#[test]
fn update_from_string() {
    let (stmt, _arena, interner) = Query::from("users")
        .update()
        .set("email")
        .filter(field("id").eq(param()))
        .returning_all()
        .build();
    match stmt {
        dol_ir::Statement::Update(upd) => {
            assert_eq!(interner.get(upd.target), "users");
            assert_eq!(upd.columns.len(), 1);
            assert_eq!(upd.values.len(), 1);
            assert_ne!(upd.filter, dol_expr::NULL_NODE);
            assert_eq!(upd.returning.len(), 1);
        }
        _ => panic!("expected Statement::Update"),
    }
}

// ── DeleteQuery ─────────────────────────────────────────────────

#[test]
fn delete_from_string() {
    let (stmt, _arena, interner) = Query::from("users")
        .delete()
        .filter(field("id").eq(param()))
        .returning_all()
        .build();
    match stmt {
        dol_ir::Statement::Delete(del) => {
            assert_eq!(interner.get(del.target), "users");
            assert_ne!(del.filter, dol_expr::NULL_NODE);
            assert_eq!(del.returning.len(), 1);
        }
        _ => panic!("expected Statement::Delete"),
    }
}

// ── UpsertQuery ─────────────────────────────────────────────────

#[test]
fn upsert_from_string() {
    let (stmt, _arena, interner) = Query::from("users")
        .upsert()
        .fields(&["id", "email", "name"])
        .on_conflict(&["id"])
        .do_update(&["email", "name"])
        .build();
    match stmt {
        dol_ir::Statement::Upsert(ups) => {
            assert_eq!(interner.get(ups.target), "users");
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
fn upsert_do_nothing() {
    let (stmt, _arena, _interner) = Query::from("users")
        .upsert()
        .fields(&["id", "email"])
        .on_conflict(&["id"])
        .do_nothing()
        .build();
    match stmt {
        dol_ir::Statement::Upsert(ups) => {
            assert!(matches!(
                ups.conflict,
                Some(dol_expr::expr::ConflictClause::DoNothing)
            ));
        }
        _ => panic!("expected Statement::Upsert"),
    }
}

// ===========================================================================
// build() tests — verify arena-based IR construction
// ===========================================================================

#[test]
fn get_build_produces_query_statement() {
    let (stmt, _arena, interner) = Query::from("users")
        .get()
        .fields(&["id", "email"])
        .filter(field("id").eq(param()))
        .build();
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
fn get_build_with_namespace() {
    let (stmt, _arena, interner) = Query::from("identity.users").get().fields(&["id"]).build();
    match stmt {
        dol_ir::Statement::Query(q) => {
            assert_eq!(interner.get(q.from), "identity.users");
            assert_eq!(q.columns.len(), 1);
        }
        _ => panic!("expected Statement::Query"),
    }
}

#[test]
fn insert_build_produces_insert_statement() {
    let (stmt, _arena, interner) = Query::from("users")
        .insert()
        .fields(&["id", "email"])
        .rows(2)
        .build();
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
fn insert_build_returning() {
    let (stmt, _arena, interner) = Query::from("users")
        .insert()
        .fields(&["id"])
        .returning_all()
        .build();
    match stmt {
        dol_ir::Statement::Insert(ins) => {
            assert_eq!(ins.returning.len(), 1);
            // The "id" column
            let ret_col = interner.get(ins.columns[0]);
            assert_eq!(ret_col, "id");
        }
        _ => panic!("expected Statement::Insert"),
    }
}

#[test]
fn update_build_produces_update_statement() {
    let (stmt, _arena, interner) = Query::from("users")
        .update()
        .set("email")
        .filter(field("id").eq(param()))
        .build();
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
fn delete_build_produces_delete_statement() {
    let (stmt, _arena, interner) = Query::from("users")
        .delete()
        .filter(field("id").eq(param()))
        .build();
    match stmt {
        dol_ir::Statement::Delete(del) => {
            assert_eq!(interner.get(del.target), "users");
            assert_ne!(del.filter, dol_expr::NULL_NODE);
        }
        _ => panic!("expected Statement::Delete"),
    }
}

#[test]
fn upsert_build_do_nothing() {
    let (stmt, _arena, interner) = Query::from("users")
        .upsert()
        .fields(&["id", "email"])
        .on_conflict(&["id"])
        .do_nothing()
        .build();
    match stmt {
        dol_ir::Statement::Upsert(ups) => {
            assert_eq!(interner.get(ups.target), "users");
            assert_eq!(ups.columns.len(), 2);
            assert!(matches!(
                ups.conflict,
                Some(dol_expr::expr::ConflictClause::DoNothing)
            ));
        }
        _ => panic!("expected Statement::Upsert"),
    }
}

#[test]
fn upsert_build_do_update() {
    let (stmt, _arena, _interner) = Query::from("users")
        .upsert()
        .fields(&["id", "email", "name"])
        .on_conflict(&["id"])
        .do_update(&["email", "name"])
        .build();
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
fn get_build_default_entity_fields() {
    let users = users_entity();
    let (stmt, _arena, interner) = Query::from(&users).get().build();
    match stmt {
        dol_ir::Statement::Query(q) => {
            assert_eq!(interner.get(q.from), "users");
            // Should select all 3 entity fields by default.
            assert_eq!(q.columns.len(), 3);
        }
        _ => panic!("expected Statement::Query"),
    }
}
