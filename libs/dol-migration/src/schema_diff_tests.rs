use super::*;

static OLD_MODEL: Entity = Entity::new(
    "users",
    &[
        Field::new("id", FieldType::Uuid).primary_key(),
        Field::new("email", FieldType::Text).unique(),
        Field::new("status", FieldType::Text).default("'active'"),
    ],
);

static NEW_MODEL: Entity = Entity::new(
    "users",
    &[
        Field::new("id", FieldType::Uuid).primary_key(),
        Field::new("email", FieldType::Text).unique(),
        Field::new("status", FieldType::Text).default("'active'"),
        Field::new("name", FieldType::Text).nullable(),
    ],
);

#[test]
fn diff_add_field() {
    let old = EntitySnapshot::from_entity(&OLD_MODEL);
    let new = EntitySnapshot::from_entity(&NEW_MODEL);
    let actions = diff_entities(&old, &new);
    assert_eq!(actions.len(), 1);
    assert!(matches!(&actions[0], AlterAction::AddField(f) if f.name == "name"));
}

#[test]
fn diff_drop_field() {
    let old = EntitySnapshot::from_entity(&NEW_MODEL); // has "name"
    let new = EntitySnapshot::from_entity(&OLD_MODEL); // no "name"
    let actions = diff_entities(&old, &new);
    assert_eq!(actions.len(), 1);
    assert!(matches!(&actions[0], AlterAction::DropField(name) if name == "name"));
}

#[test]
fn diff_change_type() {
    static V1: Entity = Entity::new(
        "t",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("count", FieldType::Int),
        ],
    );
    static V2: Entity = Entity::new(
        "t",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("count", FieldType::BigInt),
        ],
    );

    let actions = diff_entities(
        &EntitySnapshot::from_entity(&V1),
        &EntitySnapshot::from_entity(&V2),
    );
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        &actions[0],
        AlterAction::AlterFieldType { name, new_type } if name == "count" && *new_type == FieldType::BigInt
    ));
}

#[test]
fn diff_change_nullability() {
    static V1: Entity = Entity::new("t", &[Field::new("name", FieldType::Text)]);
    static V2: Entity = Entity::new("t", &[Field::new("name", FieldType::Text).nullable()]);

    let actions = diff_entities(
        &EntitySnapshot::from_entity(&V1),
        &EntitySnapshot::from_entity(&V2),
    );
    assert_eq!(actions.len(), 1);
    assert!(matches!(&actions[0], AlterAction::DropFieldNotNull(n) if n == "name"));
}

#[test]
fn diff_change_default() {
    static V1: Entity = Entity::new("t", &[Field::new("status", FieldType::Text)]);
    static V2: Entity = Entity::new(
        "t",
        &[Field::new("status", FieldType::Text).default("'new'")],
    );

    let actions = diff_entities(
        &EntitySnapshot::from_entity(&V1),
        &EntitySnapshot::from_entity(&V2),
    );
    assert_eq!(actions.len(), 1);
    assert!(
        matches!(&actions[0], AlterAction::SetFieldDefault { name, expr } if name == "status" && expr == "'new'")
    );
}

#[test]
fn diff_drop_default() {
    static V1: Entity = Entity::new(
        "t",
        &[Field::new("status", FieldType::Text).default("'old'")],
    );
    static V2: Entity = Entity::new("t", &[Field::new("status", FieldType::Text)]);

    let actions = diff_entities(
        &EntitySnapshot::from_entity(&V1),
        &EntitySnapshot::from_entity(&V2),
    );
    assert_eq!(actions.len(), 1);
    assert!(matches!(&actions[0], AlterAction::DropFieldDefault(n) if n == "status"));
}

#[test]
fn diff_no_changes() {
    let snap = EntitySnapshot::from_entity(&OLD_MODEL);
    let actions = diff_entities(&snap, &snap.clone());
    assert!(actions.is_empty());
}

#[test]
fn diff_multiple_changes() {
    static V1: Entity = Entity::new(
        "items",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("name", FieldType::Text),
            Field::new("removed_field", FieldType::Text),
        ],
    );
    static V2: Entity = Entity::new(
        "items",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("name", FieldType::Varchar(Some(255))),
            Field::new("added_field", FieldType::Int).nullable(),
        ],
    );

    let actions = diff_entities(
        &EntitySnapshot::from_entity(&V1),
        &EntitySnapshot::from_entity(&V2),
    );
    // AddField(added_field), DropField(removed_field), AlterFieldType(name)
    assert_eq!(actions.len(), 3);
}

#[test]
fn diff_to_steps_produces_alter() {
    let old = EntitySnapshot::from_entity(&OLD_MODEL);
    let new = EntitySnapshot::from_entity(&NEW_MODEL);
    let steps = diff_to_steps("users", &old, &new);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].kind(), "sql");
}

#[test]
fn diff_to_steps_empty_when_no_changes() {
    let snap = EntitySnapshot::from_entity(&OLD_MODEL);
    let steps = diff_to_steps("users", &snap, &snap.clone());
    assert!(steps.is_empty());
}

#[test]
fn create_model_step_works() {
    let step = create_entity_step(&OLD_MODEL);
    assert_eq!(step.kind(), "sql");
}

#[test]
fn drop_model_step_works() {
    let step = drop_entity_step(&OLD_MODEL);
    assert_eq!(step.kind(), "sql");
}

#[test]
fn field_to_field_def_preserves_attributes() {
    let field = Field::new("email", FieldType::Text)
        .unique()
        .nullable()
        .default("'test'")
        .index();
    let def = field_to_field_def(&field);
    assert_eq!(def.name, "email");
    assert_eq!(def.field_type, FieldType::Text);
    assert!(def.unique);
    assert!(def.nullable);
    assert_eq!(def.default_expr.as_deref(), Some("'test'"));
    assert!(def.indexed);
}

#[test]
fn snapshot_from_field_defs() {
    let fields = vec![
        FieldDef::new("id", FieldType::Uuid).primary_key(),
        FieldDef::new("name", FieldType::Text),
    ];
    let snap = EntitySnapshot::from_field_defs("test", &fields);
    assert_eq!(snap.name, "test");
    assert_eq!(snap.fields.len(), 2);
    assert!(snap.fields[0].primary_key);
}
