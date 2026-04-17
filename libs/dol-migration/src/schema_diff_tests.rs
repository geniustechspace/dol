use super::*;

fn old_model() -> Entity {
    Entity::new(
        "users",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("email", DataType::Text).unique(),
            Field::new("status", DataType::Text).default("'active'"),
        ],
    )
}

fn new_model() -> Entity {
    Entity::new(
        "users",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("email", DataType::Text).unique(),
            Field::new("status", DataType::Text).default("'active'"),
            Field::new("name", DataType::Text).nullable(),
        ],
    )
}

#[test]
fn diff_add_field() {
    let old = EntitySnapshot::from_entity(&old_model());
    let new = EntitySnapshot::from_entity(&new_model());
    let actions = diff_entities(&old, &new);
    assert_eq!(actions.len(), 1);
    assert!(matches!(&actions[0], AlterAction::AddField(f) if f.name == "name"));
}

#[test]
fn diff_drop_field() {
    let old = EntitySnapshot::from_entity(&new_model()); // has "name"
    let new = EntitySnapshot::from_entity(&old_model()); // no "name"
    let actions = diff_entities(&old, &new);
    assert_eq!(actions.len(), 1);
    assert!(matches!(&actions[0], AlterAction::DropField(name) if name == "name"));
}

#[test]
fn diff_change_type() {
    let v1 = Entity::new(
        "t",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("count", DataType::Int32),
        ],
    );
    let v2 = Entity::new(
        "t",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("count", DataType::Int64),
        ],
    );

    let actions = diff_entities(
        &EntitySnapshot::from_entity(&v1),
        &EntitySnapshot::from_entity(&v2),
    );
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        &actions[0],
        AlterAction::AlterFieldType { name, new_type } if name == "count" && *new_type == DataType::Int64
    ));
}

#[test]
fn diff_change_nullability() {
    let v1 = Entity::new("t", vec![Field::new("name", DataType::Text)]);
    let v2 = Entity::new("t", vec![Field::new("name", DataType::Text).nullable()]);

    let actions = diff_entities(
        &EntitySnapshot::from_entity(&v1),
        &EntitySnapshot::from_entity(&v2),
    );
    assert_eq!(actions.len(), 1);
    assert!(matches!(&actions[0], AlterAction::DropFieldNotNull(n) if n == "name"));
}

#[test]
fn diff_change_default() {
    let v1 = Entity::new("t", vec![Field::new("status", DataType::Text)]);
    let v2 = Entity::new(
        "t",
        vec![Field::new("status", DataType::Text).default("'new'")],
    );

    let actions = diff_entities(
        &EntitySnapshot::from_entity(&v1),
        &EntitySnapshot::from_entity(&v2),
    );
    assert_eq!(actions.len(), 1);
    assert!(
        matches!(&actions[0], AlterAction::SetFieldDefault { name, expr } if name == "status" && expr == "'new'")
    );
}

#[test]
fn diff_drop_default() {
    let v1 = Entity::new(
        "t",
        vec![Field::new("status", DataType::Text).default("'old'")],
    );
    let v2 = Entity::new("t", vec![Field::new("status", DataType::Text)]);

    let actions = diff_entities(
        &EntitySnapshot::from_entity(&v1),
        &EntitySnapshot::from_entity(&v2),
    );
    assert_eq!(actions.len(), 1);
    assert!(matches!(&actions[0], AlterAction::DropFieldDefault(n) if n == "status"));
}

#[test]
fn diff_no_changes() {
    let snap = EntitySnapshot::from_entity(&old_model());
    let actions = diff_entities(&snap, &snap.clone());
    assert!(actions.is_empty());
}

#[test]
fn diff_multiple_changes() {
    let v1 = Entity::new(
        "items",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("name", DataType::Text),
            Field::new("removed_field", DataType::Text),
        ],
    );
    let v2 = Entity::new(
        "items",
        vec![
            Field::new("id", DataType::Uuid).primary_key(),
            Field::new("name", DataType::Varchar(Some(255))),
            Field::new("added_field", DataType::Int32).nullable(),
        ],
    );

    let actions = diff_entities(
        &EntitySnapshot::from_entity(&v1),
        &EntitySnapshot::from_entity(&v2),
    );
    // AddField(added_field), DropField(removed_field), AlterFieldType(name)
    assert_eq!(actions.len(), 3);
}

#[test]
fn diff_to_steps_produces_alter() {
    let old = EntitySnapshot::from_entity(&old_model());
    let new = EntitySnapshot::from_entity(&new_model());
    let steps = diff_to_steps("users", &old, &new);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].kind(), "sql");
}

#[test]
fn diff_to_steps_empty_when_no_changes() {
    let snap = EntitySnapshot::from_entity(&old_model());
    let steps = diff_to_steps("users", &snap, &snap.clone());
    assert!(steps.is_empty());
}

#[test]
fn create_model_step_works() {
    let step = create_entity_step(&old_model());
    assert_eq!(step.kind(), "sql");
}

#[test]
fn drop_model_step_works() {
    let step = drop_entity_step(&old_model());
    assert_eq!(step.kind(), "sql");
}

#[test]
fn field_to_field_def_preserves_attributes() {
    let field = Field::new("email", DataType::Text)
        .unique()
        .nullable()
        .default("'test'")
        .index();
    let def = field_to_field_def(&field);
    assert_eq!(def.name, "email");
    assert_eq!(def.data_type, DataType::Text);
    assert!(def.unique);
    assert!(def.nullable);
    assert_eq!(def.default_expr.as_deref(), Some("'test'"));
    assert!(def.indexed);
}

#[test]
fn snapshot_from_field_defs() {
    let fields = vec![
        FieldDef::new("id", DataType::Uuid).primary_key(),
        FieldDef::new("name", DataType::Text),
    ];
    let snap = EntitySnapshot::from_field_defs("test", &fields);
    assert_eq!(snap.name, "test");
    assert_eq!(snap.fields.len(), 2);
    assert!(snap.fields[0].primary_key);
}
