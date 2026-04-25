//! Round-trip serde tests for `dol-schema`.
//!
//! These tests guarantee that every public schema type can be serialised to
//! JSON and deserialised back into an equal value. This is the property the
//! crate's owned-string sweep was designed to enable.

#![cfg(feature = "serde")]

use dol_schema::{DataType, Entity, EntityConstraint, Field, FkAction, ForeignKeyRef};

fn round_trip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let json = serde_json::to_string(value).expect("serialize");
    serde_json::from_str(&json).expect("deserialize")
}

#[test]
fn fk_action_round_trip() {
    for action in [
        FkAction::NoAction,
        FkAction::Cascade,
        FkAction::SetNull,
        FkAction::Restrict,
        FkAction::SetDefault,
    ] {
        assert_eq!(action, round_trip(&action));
    }
}

#[test]
fn foreign_key_ref_round_trip() {
    let fk = ForeignKeyRef::new("users", "id")
        .on_delete(FkAction::Cascade)
        .on_update(FkAction::SetNull);
    assert_eq!(fk, round_trip(&fk));
}

#[test]
fn entity_constraint_unique_round_trip() {
    let c = EntityConstraint::unique(["tenant_id", "email"]);
    assert_eq!(c, round_trip(&c));
}

#[test]
fn entity_constraint_foreign_key_round_trip() {
    let c = EntityConstraint::foreign_key(["user_id"], "users", ["id"], FkAction::Cascade);
    assert_eq!(c, round_trip(&c));
}

#[test]
fn entity_constraint_check_round_trip() {
    let c = EntityConstraint::check("age > 0");
    assert_eq!(c, round_trip(&c));
}

#[test]
fn entity_constraint_primary_key_round_trip() {
    let c = EntityConstraint::primary_key(["tenant_id", "user_id"]);
    assert_eq!(c, round_trip(&c));
}

#[test]
fn field_round_trip_minimal() {
    let f = Field::new("id", DataType::Uuid).primary_key();
    assert_eq!(f, round_trip(&f));
}

#[test]
fn field_round_trip_full() {
    let f = Field::new("user_id", DataType::Uuid)
        .references("users", "id", FkAction::Cascade, FkAction::NoAction)
        .check("user_id IS NOT NULL")
        .comment("FK to users.id")
        .collation("C")
        .default("gen_random_uuid()")
        .index();
    assert_eq!(f, round_trip(&f));
}

#[test]
fn field_round_trip_generated() {
    let f = Field::new("total", DataType::Int32).generated_stored("price * qty");
    assert_eq!(f, round_trip(&f));
}

#[test]
fn entity_round_trip_simple() {
    let e = Entity::new(
        "users",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("email", DataType::unbounded_string()).unique(),
        ],
    );
    assert_eq!(e, round_trip(&e));
}

#[test]
fn entity_round_trip_with_namespace_and_constraints() {
    let e = Entity::new(
        "orders",
        vec![
            Field::new("id", DataType::Int64)
                .primary_key()
                .auto_increment(),
            Field::new("user_id", DataType::Uuid),
            Field::new("product", DataType::unbounded_string()),
        ],
    )
    .with_namespace("public")
    .with_constraints(vec![
        EntityConstraint::unique(["user_id", "product"]),
        EntityConstraint::foreign_key(["user_id"], "users", ["id"], FkAction::Cascade),
        EntityConstraint::check("user_id IS NOT NULL"),
    ]);
    assert_eq!(e, round_trip(&e));
}
