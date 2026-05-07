//! Integration tests for `dol-query` builders against the
//! [`Operation`](dol_ir::operation::Operation) IR.

use alloc::{boxed::Box, format, string::String, vec};

use super::*;
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

fn target_name(p: &dol_ir::program::Program) -> &str {
    let op = &p.operations[0];
    let target = op.primary_target().expect("operation has a target");
    let name = p.interner.get(target.locator.name.0);
    if let Some(ns) = target.locator.namespace {
        let ns_str = p.interner.get(ns.0);
        // Returning a borrowed slice of the interner buffer doesn't compose
        // cleanly with the namespace; build a static cell via leak. For
        // tests this is acceptable.
        return Box::leak(format!("{ns_str}.{name}").into_boxed_str());
    }
    name
}

/// Pull the inner arena `QueryNode` out of a `Program` whose first
/// operation is `Operation::Query`.
fn unwrap_query(p: &dol_ir::program::Program) -> &dol_expr::expr::QueryNode {
    match &p.operations[0] {
        dol_ir::operation::Operation::Query(q) => {
            let body = q.node.expect("Query has arena body");
            let qid = p.arena.get(body).as_query().expect("expected Query opcode");
            p.arena.get_query(qid)
        }
        other => panic!("expected Operation::Query, got {other:?}"),
    }
}

fn unwrap_insert(p: &dol_ir::program::Program) -> &dol_expr::expr::InsertNode {
    match &p.operations[0] {
        dol_ir::operation::Operation::Insert(ins) => {
            let body = match ins.source {
                dol_ir::operation::InsertSource::Node(n) => n,
                _ => panic!("expected InsertSource::Node"),
            };
            let iid = p
                .arena
                .get(body)
                .as_insert()
                .expect("expected Insert opcode");
            p.arena.get_insert(iid)
        }
        other => panic!("expected Operation::Insert, got {other:?}"),
    }
}

fn unwrap_update(p: &dol_ir::program::Program) -> &dol_expr::expr::UpdateNode {
    match &p.operations[0] {
        dol_ir::operation::Operation::Update(u) => {
            let uid = p
                .arena
                .get(u.node)
                .as_update()
                .expect("expected Update opcode");
            p.arena.get_update(uid)
        }
        other => panic!("expected Operation::Update, got {other:?}"),
    }
}

fn unwrap_delete(p: &dol_ir::program::Program) -> &dol_expr::expr::DeleteNode {
    match &p.operations[0] {
        dol_ir::operation::Operation::Delete(d) => {
            let did = p
                .arena
                .get(d.node)
                .as_delete()
                .expect("expected Delete opcode");
            p.arena.get_delete(did)
        }
        other => panic!("expected Operation::Delete, got {other:?}"),
    }
}

fn unwrap_upsert(p: &dol_ir::program::Program) -> &dol_expr::expr::UpsertNode {
    match &p.operations[0] {
        dol_ir::operation::Operation::Upsert(u) => {
            let uid = p
                .arena
                .get(u.node)
                .as_upsert()
                .expect("expected Upsert opcode");
            p.arena.get_upsert(uid)
        }
        other => panic!("expected Operation::Upsert, got {other:?}"),
    }
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
    let q = Query::from("identity.users").namespace("profiles");
    assert_eq!(q.name, "profiles");
    assert_eq!(q.namespace.as_deref(), Some("identity.users"));
}

#[test]
fn namespace_propagates_to_get_ir() {
    let p = Query::from("api")
        .namespace("v1")
        .namespace("users")
        .get()
        .fields(&["id"])
        .try_build()
        .expect("test fixture: builder must succeed");
    let q = unwrap_query(&p);
    assert_eq!(p.interner.get(q.from), "api.v1.users");
    assert_eq!(target_name(&p), "api.v1.users");
}

#[test]
fn namespace_propagates_to_insert_ir() {
    let p = Query::from("api")
        .namespace("users")
        .insert()
        .fields(&["id", "email"])
        .try_build()
        .expect("test fixture: builder must succeed");
    let ins = unwrap_insert(&p);
    assert_eq!(p.interner.get(ins.target), "api.users");
    assert_eq!(target_name(&p), "api.users");
}

#[test]
fn namespace_propagates_to_update_ir() {
    let p = Query::from("api")
        .namespace("users")
        .update()
        .set("email")
        .try_build()
        .expect("test fixture: builder must succeed");
    let upd = unwrap_update(&p);
    assert_eq!(p.interner.get(upd.target), "api.users");
}

#[test]
fn namespace_propagates_to_delete_ir() {
    let p = Query::from("api")
        .namespace("users")
        .delete()
        .filter(dol_expr::tree::field("id").eq(dol_expr::tree::param()))
        .try_build()
        .expect("test fixture: builder must succeed");
    let del = unwrap_delete(&p);
    assert_eq!(p.interner.get(del.target), "api.users");
}

#[test]
fn namespace_propagates_to_upsert_ir() {
    let p = Query::from("api")
        .namespace("users")
        .upsert()
        .fields(&["id", "email"])
        .match_on(&["id"])
        .then_skip()
        .try_build()
        .expect("test fixture: builder must succeed");
    let ups = unwrap_upsert(&p);
    assert_eq!(p.interner.get(ups.target), "api.users");
}

// ── GetQuery from Entity ────────────────────────────────────────

#[test]
fn get_from_entity_defaults() {
    let users = users_entity();
    let p = Query::from(&users)
        .get()
        .try_build()
        .expect("test fixture: builder must succeed");
    let q = unwrap_query(&p);
    assert_eq!(p.interner.get(q.from), "users");
    assert_eq!(q.columns.len(), 3);
}

#[test]
fn get_from_entity_filter() {
    let users = users_entity();
    let p = Query::from(&users)
        .get()
        .filter(dol_expr::tree::field("id").eq(dol_expr::tree::param()))
        .try_build()
        .expect("test fixture: builder must succeed");
    let q = unwrap_query(&p);
    assert!(q.filter.is_some());
}

// ── GetQuery from string ────────────────────────────────────────

#[test]
fn get_from_string_columns() {
    let p = Query::from("users")
        .get()
        .fields(&["id", "email"])
        .filter(dol_expr::tree::field("id").eq(dol_expr::tree::param()))
        .try_build()
        .expect("test fixture: builder must succeed");
    let q = unwrap_query(&p);
    assert_eq!(p.interner.get(q.from), "users");
    assert_eq!(q.columns.len(), 2);
    assert!(q.filter.is_some());
}

#[test]
fn get_from_namespaced_string() {
    let p = Query::from("identity.users")
        .get()
        .fields(&["id"])
        .try_build()
        .expect("test fixture: builder must succeed");
    let q = unwrap_query(&p);
    assert_eq!(p.interner.get(q.from), "identity.users");
}

// ── InsertQuery ─────────────────────────────────────────────────

#[test]
fn insert_from_entity_defaults() {
    let users = users_entity();
    let p = Query::from(&users)
        .insert()
        .try_build()
        .expect("test fixture: builder must succeed");
    let ins = unwrap_insert(&p);
    assert_eq!(p.interner.get(ins.target), "users");
    assert_eq!(ins.columns.len(), 3);
}

#[test]
fn insert_from_string_columns() {
    let p = Query::from("users")
        .insert()
        .fields(&["id", "email"])
        .rows(2)
        .returning_all()
        .try_build()
        .expect("test fixture: builder must succeed");
    let ins = unwrap_insert(&p);
    assert_eq!(p.interner.get(ins.target), "users");
    assert_eq!(ins.columns.len(), 2);
    assert_eq!(ins.values.len(), 4);
    assert_eq!(ins.returning.len(), 1);
}

// ── UpdateQuery ─────────────────────────────────────────────────

#[test]
fn update_from_string() {
    let p = Query::from("users")
        .update()
        .set("email")
        .filter(dol_expr::tree::field("id").eq(dol_expr::tree::param()))
        .returning_all()
        .try_build()
        .expect("test fixture: builder must succeed");
    let upd = unwrap_update(&p);
    assert_eq!(p.interner.get(upd.target), "users");
    assert_eq!(upd.columns.len(), 1);
    assert_eq!(upd.values.len(), 1);
    assert!(upd.filter.is_some());
    assert_eq!(upd.returning.len(), 1);
}

// ── DeleteQuery ─────────────────────────────────────────────────

#[test]
fn delete_from_string() {
    let p = Query::from("users")
        .delete()
        .filter(dol_expr::tree::field("id").eq(dol_expr::tree::param()))
        .returning_all()
        .try_build()
        .expect("test fixture: builder must succeed");
    let del = unwrap_delete(&p);
    assert_eq!(p.interner.get(del.target), "users");
    assert!(del.filter.is_some());
    assert_eq!(del.returning.len(), 1);
}

// ── UpsertQuery ─────────────────────────────────────────────────

#[test]
fn upsert_from_string() {
    let p = Query::from("users")
        .upsert()
        .fields(&["id", "email", "name"])
        .match_on(&["id"])
        .then_patch(&["email", "name"])
        .try_build()
        .expect("test fixture: builder must succeed");
    let ups = unwrap_upsert(&p);
    assert_eq!(p.interner.get(ups.target), "users");
    assert_eq!(ups.columns.len(), 3);
    match &ups.conflict {
        Some(dol_expr::expr::ConflictClause::DoUpdate { assignments }) => {
            assert_eq!(assignments.len(), 2);
        }
        _ => panic!("expected DoUpdate conflict"),
    }
}

#[test]
fn upsert_do_nothing() {
    let p = Query::from("users")
        .upsert()
        .fields(&["id", "email"])
        .match_on(&["id"])
        .then_skip()
        .try_build()
        .expect("test fixture: builder must succeed");
    let ups = unwrap_upsert(&p);
    assert!(matches!(
        ups.conflict,
        Some(dol_expr::expr::ConflictClause::DoNothing)
    ));
}

// ── op-kind dispatch ────────────────────────────────────────────

#[test]
fn get_emits_query_operation() {
    let p = Query::from("users")
        .get()
        .fields(&["id"])
        .try_build()
        .expect("test fixture: builder must succeed");
    assert_eq!(p.operations[0].kind(), dol_ir::operation::OpKind::Query);
    assert_eq!(p.operations[0].category(), dol_ir::operation::Category::DQL);
}

#[test]
fn insert_emits_insert_operation() {
    let p = Query::from("users")
        .insert()
        .fields(&["id"])
        .try_build()
        .expect("test fixture: builder must succeed");
    assert_eq!(p.operations[0].kind(), dol_ir::operation::OpKind::Insert);
    assert_eq!(p.operations[0].category(), dol_ir::operation::Category::DML);
}

#[test]
fn upsert_emits_upsert_operation_and_merge_capability() {
    let p = Query::from("users")
        .upsert()
        .fields(&["id"])
        .match_on(&["id"])
        .then_skip()
        .try_build()
        .expect("test fixture: builder must succeed");
    assert_eq!(p.operations[0].kind(), dol_ir::operation::OpKind::Upsert);
    assert!(
        p.operations[0]
            .required_capabilities()
            .contains(&dol_ir::capabilities::CapabilityTag::MERGE)
    );
}
