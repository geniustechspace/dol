use super::*;

// ── Helpers ─────────────────────────────────────────────────────────

fn basic_fields() -> Vec<Field> {
    vec![
        Field::new("id", DataType::Uuid).primary_key(),
        Field::new("name", DataType::Text),
        Field::new("email", DataType::Text).nullable(),
    ]
}

fn basic_model() -> Entity {
    Entity::new("users", basic_fields())
}

fn ns_model() -> Entity {
    Entity::new("users", basic_fields()).with_namespace("public")
}

fn constrained_model() -> Entity {
    Entity::new(
        "orders",
        vec![
            Field::new("id", DataType::Int32)
                .primary_key()
                .auto_increment(),
            Field::new("user_id", DataType::Uuid),
            Field::new("product", DataType::Text),
        ],
    )
    .with_constraints(vec![
        EntityConstraint::Unique(&["user_id", "product"]),
        EntityConstraint::Check("user_id IS NOT NULL"),
    ])
}

// ── 1. Entity::new ───────────────────────────────────────────────────

#[test]
fn model_new_defaults() {
    let m = Entity::new("items", vec![]);
    assert_eq!(m.name, "items");
    assert_eq!(m.namespace, None);
    assert_eq!(m.fields.len(), 0);
    assert_eq!(m.constraints.len(), 0);
}

// ── 2. Entity::with_namespace ────────────────────────────────────────

#[test]
fn model_with_namespace() {
    let m = ns_model();
    assert_eq!(m.namespace, Some("public"));
    assert_eq!(m.name, "users");
}

// ── 3. Entity::with_constraints ──────────────────────────────────────

#[test]
fn model_with_constraints() {
    let m = constrained_model();
    assert_eq!(m.constraints.len(), 2);
    assert_eq!(
        m.constraints[0],
        EntityConstraint::Unique(&["user_id", "product"]),
    );
    assert_eq!(
        m.constraints[1],
        EntityConstraint::Check("user_id IS NOT NULL"),
    );
}

// ── 4. Entity::qualified_name ────────────────────────────────────────

#[test]
fn qualified_name_without_namespace() {
    assert_eq!(basic_model().qualified_name(), "users");
}

#[test]
fn qualified_name_with_namespace() {
    assert_eq!(ns_model().qualified_name(), "public.users");
}

// ── 5 & 6. Entity::field ─────────────────────────────────────────────

#[test]
fn field_lookup_success() {
    let m = basic_model();
    let f = m.field("email");
    assert_eq!(f.name, "email");
    assert!(f.nullable);
}

#[test]
#[should_panic(expected = "field 'missing' not found in entity 'users'")]
fn field_lookup_panic_on_missing() {
    basic_model().field("missing");
}

// ── 7. Entity::try_field ─────────────────────────────────────────────

#[test]
fn try_field_some() {
    assert!(basic_model().try_field("id").is_some());
}

#[test]
fn try_field_none() {
    assert!(basic_model().try_field("nonexistent").is_none());
}

// ── 8. Entity::field_names ───────────────────────────────────────────

#[test]
fn field_names_iterator() {
    let names: Vec<_> = basic_model().field_names().collect();
    assert_eq!(names, vec!["id", "name", "email"]);
}

// ── 9. Entity::primary_keys ──────────────────────────────────────────

#[test]
fn primary_keys_filter() {
    let m = basic_model();
    let pks: Vec<_> = m.primary_keys().collect();
    assert_eq!(pks.len(), 1);
    assert_eq!(pks[0].name, "id");
    assert!(pks[0].primary_key);
}

// ── 10. Entity::non_pk_fields ────────────────────────────────────────

#[test]
fn non_pk_fields_filter() {
    let m = basic_model();
    let non_pk: Vec<_> = m.non_pk_fields().collect();
    assert_eq!(non_pk.len(), 2);
    assert_eq!(non_pk[0].name, "name");
    assert_eq!(non_pk[1].name, "email");
}

// ── 11. Entity::field_list ───────────────────────────────────────────

#[test]
fn field_list_comma_separated() {
    assert_eq!(basic_model().field_list(), "id, name, email");
}

#[test]
fn field_list_empty_model() {
    let m = Entity::new("empty", vec![]);
    assert_eq!(m.field_list(), "");
}

// ── 12. Field::new defaults ─────────────────────────────────────────

#[test]
fn field_new_defaults() {
    let f = Field::new("col", DataType::Int32);
    assert_eq!(f.name, "col");
    assert_eq!(f.data_type, DataType::Int32);
    assert!(!f.primary_key);
    assert!(!f.nullable);
    assert!(!f.has_default);
    assert_eq!(f.default_expr, None);
    assert!(!f.unique);
    assert_eq!(f.references, None);
    assert_eq!(f.check, None);
    assert_eq!(f.comment, None);
    assert_eq!(f.collation, None);
    assert_eq!(f.generated, None);
    assert!(!f.indexed);
    assert!(!f.auto_increment);
}

// ── 13. Field builder chain ─────────────────────────────────────────

#[test]
fn field_builder_chain() {
    let f = Field::new("status", DataType::Text)
        .primary_key()
        .nullable()
        .unique()
        .default("'active'")
        .check("status IN ('active','inactive')")
        .comment("User account status")
        .collation("C");

    assert!(f.primary_key);
    assert!(f.nullable);
    assert!(f.unique);
    assert!(f.has_default);
    assert_eq!(f.default_expr, Some("'active'"));
    assert_eq!(f.check, Some("status IN ('active','inactive')"));
    assert_eq!(f.comment, Some("User account status"));
    assert_eq!(f.collation, Some("C"));
}

// ── 14. Field::optional ─────────────────────────────────────────────

#[test]
fn field_optional_is_nullable() {
    let f = Field::new("bio", DataType::Text).optional();
    assert!(f.nullable);
}

// ── 15. Field::required ─────────────────────────────────────────────

#[test]
fn field_required_is_noop() {
    let f = Field::new("bio", DataType::Text).required();
    assert!(!f.nullable);
    assert_eq!(f, Field::new("bio", DataType::Text));
}

// ── 16. Field::has_default (metadata only) ──────────────────────────

#[test]
fn field_has_default_metadata() {
    let f = Field::new("seq", DataType::Int32).has_default();
    assert!(f.has_default);
    assert_eq!(f.default_expr, None);
}

#[test]
fn field_default_sets_expr() {
    let f = Field::new("ts", DataType::TimestampTz { precision: 6 }).default("NOW()");
    assert!(f.has_default);
    assert_eq!(f.default_expr, Some("NOW()"));
}

// ── Field::references ───────────────────────────────────────────────

#[test]
fn field_references_inline() {
    let f = Field::new("user_id", DataType::Uuid).references(
        "users",
        "id",
        FkAction::Cascade,
        FkAction::NoAction,
    );
    let fk = f.references.unwrap();
    assert_eq!(fk.table, "users");
    assert_eq!(fk.column, "id");
    assert_eq!(fk.on_delete, FkAction::Cascade);
    assert_eq!(fk.on_update, FkAction::NoAction);
}

#[test]
fn field_references_full_from_builder() {
    let fk = ForeignKeyRef::new("orgs", "org_id")
        .on_delete(FkAction::SetNull)
        .on_update(FkAction::Restrict);
    let f = Field::new("org_id", DataType::Uuid).references_full(fk);
    let got = f.references.unwrap();
    assert_eq!(got.table, "orgs");
    assert_eq!(got.column, "org_id");
    assert_eq!(got.on_delete, FkAction::SetNull);
    assert_eq!(got.on_update, FkAction::Restrict);
}

// ── Field::generated_stored / generated_virtual ─────────────────────

#[test]
fn field_generated_stored() {
    let f = Field::new("total", DataType::Int32).generated_stored("price * qty");
    assert_eq!(f.generated, Some((GeneratedKind::Stored, "price * qty")));
}

#[test]
fn field_generated_virtual() {
    let f = Field::new("full_name", DataType::Text).generated_virtual("first || ' ' || last");
    assert_eq!(
        f.generated,
        Some((GeneratedKind::Virtual, "first || ' ' || last")),
    );
}

// ── Field::index ────────────────────────────────────────────────────

#[test]
fn field_index_hint() {
    let f = Field::new("email", DataType::Text).index();
    assert!(f.indexed);
}

// ── Field::auto_increment ───────────────────────────────────────────

#[test]
fn field_auto_increment() {
    let f = Field::new("id", DataType::Int32)
        .auto_increment()
        .primary_key();
    assert!(f.auto_increment);
    assert!(f.primary_key);
    assert_eq!(f.data_type, DataType::Int32);
}

#[test]
fn field_auto_increment_bigint() {
    let f = Field::new("id", DataType::Int64).auto_increment();
    assert!(f.auto_increment);
    assert_eq!(f.data_type, DataType::Int64);
}

// ── FkAction Display ────────────────────────────────────────────────

#[test]
fn fk_action_display() {
    assert_eq!(FkAction::NoAction.to_string(), "NO ACTION");
    assert_eq!(FkAction::Cascade.to_string(), "CASCADE");
    assert_eq!(FkAction::SetNull.to_string(), "SET NULL");
    assert_eq!(FkAction::Restrict.to_string(), "RESTRICT");
    assert_eq!(FkAction::SetDefault.to_string(), "SET DEFAULT");
}

// ── GeneratedKind ───────────────────────────────────────────────────

#[test]
fn generated_kind_debug_clone_copy() {
    let stored = GeneratedKind::Stored;
    let copied = stored; // Copy
    assert_eq!(stored, copied);
    assert_eq!(format!("{:?}", GeneratedKind::Stored), "Stored");
    assert_eq!(format!("{:?}", GeneratedKind::Virtual), "Virtual");
}

// ── ForeignKeyRef ───────────────────────────────────────────────────

#[test]
fn foreign_key_ref_defaults() {
    let fk = ForeignKeyRef::new("users", "id");
    assert_eq!(fk.table, "users");
    assert_eq!(fk.column, "id");
    assert_eq!(fk.on_delete, FkAction::NoAction);
    assert_eq!(fk.on_update, FkAction::NoAction);
}

#[test]
fn foreign_key_ref_builder() {
    let fk = ForeignKeyRef::new("users", "id")
        .on_delete(FkAction::Cascade)
        .on_update(FkAction::SetDefault);
    assert_eq!(fk.on_delete, FkAction::Cascade);
    assert_eq!(fk.on_update, FkAction::SetDefault);
}

// ── EntityConstraint variants ───────────────────────────────────────

#[test]
fn model_constraint_unique() {
    let c = EntityConstraint::Unique(&["a", "b"]);
    assert_eq!(c, EntityConstraint::Unique(&["a", "b"]));
}

#[test]
fn model_constraint_foreign_key() {
    let c = EntityConstraint::ForeignKey {
        columns: &["user_id"],
        ref_table: "users",
        ref_columns: &["id"],
        on_delete: FkAction::Cascade,
    };
    if let EntityConstraint::ForeignKey {
        columns,
        ref_table,
        ref_columns,
        on_delete,
    } = c
    {
        assert_eq!(columns, &["user_id"]);
        assert_eq!(ref_table, "users");
        assert_eq!(ref_columns, &["id"]);
        assert_eq!(on_delete, FkAction::Cascade);
    } else {
        panic!("expected ForeignKey variant");
    }
}

#[test]
fn model_constraint_check() {
    let c = EntityConstraint::Check("age > 0");
    assert_eq!(c, EntityConstraint::Check("age > 0"));
}

#[test]
fn model_constraint_primary_key() {
    let c = EntityConstraint::PrimaryKey(&["tenant_id", "user_id"]);
    assert_eq!(c, EntityConstraint::PrimaryKey(&["tenant_id", "user_id"]));
}

// ── Entity equality ─────────────────────────────────────────────────

#[test]
fn model_equality() {
    let a = Entity::new("t", vec![]);
    let b = Entity::new("t", vec![]);
    assert_eq!(a, b);
}

#[test]
fn model_inequality() {
    let a = Entity::new("t1", vec![]);
    let b = Entity::new("t2", vec![]);
    assert_ne!(a, b);
}

// ── DataType used through entity ────────────────────────────────────

#[test]
fn data_type_reexported() {
    // Verify DataType is accessible through dol-entity
    let _ = DataType::Text;
    let _ = DataType::Uuid;
    let _ = DataType::Int32;
    let _ = DataType::Varchar(Some(255));
    let _ = DataType::Json;
    let _ = DataType::TimestampTz { precision: 6 };
    let _ = DataType::Array(Box::new(DataType::Text));
}
