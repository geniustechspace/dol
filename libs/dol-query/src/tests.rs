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
    assert!(matches!(&ir.projections[0], Expr::Identifier(n) if n == "id"));
    assert!(matches!(&ir.projections[2], Expr::Identifier(n) if n == "name"));
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
