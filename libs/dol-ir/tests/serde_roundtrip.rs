//! Round-trip serde tests for `dol-ir` owned (non-arena) types.
//!
//! Verifies that the DDL, storage, transaction, and control variants
//! survive a JSON encode → decode cycle.

#![cfg(feature = "serde")]

use dol_ir::definition::{
    AlterAction, AlterEntity, DefineEntity, DefineIndex, DefineType, DropEntity, DropIndex,
    DropType, FieldDef, IndexMethod,
};
use dol_ir::entity_ref::EntityRef;
use dol_ir::{EntityConstraint, RefAction, RelationRef};
use dol_types::DataType;

fn round_trip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let json = serde_json::to_string(value).expect("serialize");
    serde_json::from_str(&json).expect("deserialize")
}

#[test]
fn entity_ref_round_trip() {
    let r = EntityRef {
        name: "users".into(),
        namespace: Some("public".into()),
        alias: Some("u".into()),
    };
    assert_eq!(r, round_trip(&r));
}

#[test]
fn field_def_round_trip() {
    let fd = FieldDef::new("id", DataType::Uuid)
        .identity()
        .default("gen_random_uuid()")
        .comment("primary key")
        .references(
            RelationRef::new("users", "id").on_delete(RefAction::Cascade),
        );
    assert_eq!(fd, round_trip(&fd));
}

#[test]
fn owned_entity_constraint_round_trip() {
    let cs = vec![
        EntityConstraint::Unique(vec!["a".into(), "b".into()]),
        EntityConstraint::Identity(vec!["id".into()]),
        EntityConstraint::Invariant("x > 0".into()),
        EntityConstraint::Relation {
            fields: vec!["user_id".into()],
            ref_entity: "users".into(),
            ref_fields: vec!["id".into()],
            on_delete: RefAction::Detach,
        },
    ];
    for c in &cs {
        assert_eq!(*c, round_trip(c));
    }
}

#[test]
fn define_entity_round_trip() {
    let de = DefineEntity {
        name: "users".into(),
        namespace: Some("public".into()),
        fields: vec![
            FieldDef::new("id", DataType::Uuid).identity(),
            FieldDef::new("email", DataType::unbounded_string()).unique(),
        ],
        constraints: vec![EntityConstraint::Unique(vec![
            "email".into(),
        ])],
        if_not_exists: true,
    };
    assert_eq!(de, round_trip(&de));
}

#[test]
fn alter_entity_round_trip() {
    let a = AlterEntity {
        target: EntityRef {
            name: "users".into(),
            namespace: None,
            alias: None,
        },
        actions: vec![
            AlterAction::AddField(FieldDef::new("age", DataType::Int32)),
            AlterAction::DropField("legacy".into()),
            AlterAction::RenameField {
                from: "old".into(),
                to: "new".into(),
            },
            AlterAction::AlterFieldType {
                name: "age".into(),
                new_type: DataType::Int64,
            },
            AlterAction::SetFieldDefault {
                name: "age".into(),
                expr: "0".into(),
            },
            AlterAction::DropFieldDefault("age".into()),
            AlterAction::SetFieldNotNull("age".into()),
            AlterAction::DropFieldNotNull("age".into()),
            AlterAction::AddConstraint(EntityConstraint::Unique(vec!["age".into()])),
            AlterAction::DropConstraint("uniq_age".into()),
            AlterAction::RenameEntity("people".into()),
        ],
    };
    assert_eq!(a, round_trip(&a));
}

#[test]
fn drop_entity_round_trip() {
    let d = DropEntity {
        target: EntityRef {
            name: "users".into(),
            namespace: None,
            alias: None,
        },
        if_exists: true,
        cascade: true,
    };
    assert_eq!(d, round_trip(&d));
}

#[test]
fn define_index_round_trip() {
    let i = DefineIndex {
        name: "idx_email".into(),
        target: EntityRef {
            name: "users".into(),
            namespace: None,
            alias: None,
        },
        columns: vec!["email".into()],
        unique: true,
        if_not_exists: true,
        concurrently: false,
        method: Some(IndexMethod::BTree),
        where_clause: Some("email IS NOT NULL".into()),
    };
    assert_eq!(i, round_trip(&i));
}

#[test]
fn drop_index_round_trip() {
    let d = DropIndex {
        name: "idx_email".into(),
        if_exists: true,
        concurrently: false,
        cascade: false,
    };
    assert_eq!(d, round_trip(&d));
}

#[test]
fn index_method_round_trip() {
    for m in [
        IndexMethod::BTree,
        IndexMethod::Hash,
        IndexMethod::FullText,
        IndexMethod::Spatial,
        IndexMethod::Custom("gin".into()),
    ] {
        assert_eq!(m, round_trip(&m));
    }
}

#[test]
fn define_type_round_trip() {
    let t = DefineType {
        name: "user_status".into(),
        namespace: Some("public".into()),
        variants: vec!["active".into(), "inactive".into(), "banned".into()],
    };
    assert_eq!(t, round_trip(&t));
}

#[test]
fn drop_type_round_trip() {
    let d = DropType {
        name: "user_status".into(),
        if_exists: true,
    };
    assert_eq!(d, round_trip(&d));
}
