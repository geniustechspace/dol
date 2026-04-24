use std::sync::Arc;

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
        EntityConstraint::unique(["user_id", "product"]),
        EntityConstraint::check("user_id IS NOT NULL"),
    ])
}

// ── 1. Entity::new ───────────────────────────────────────────────────

#[test]
fn model_new_defaults() {
    let m = Entity::new("items", vec![]);
    assert_eq!(&*m.name, "items");
    assert!(m.namespace.is_none());
    assert_eq!(m.fields.len(), 0);
    assert_eq!(m.constraints.len(), 0);
}

// ── 2. Entity::with_namespace ────────────────────────────────────────

#[test]
fn model_with_namespace() {
    let m = ns_model();
    assert_eq!(m.namespace.as_deref(), Some("public"));
    assert_eq!(&*m.name, "users");
}

// ── 3. Entity::with_constraints ──────────────────────────────────────

#[test]
fn model_with_constraints() {
    let m = constrained_model();
    assert_eq!(m.constraints.len(), 2);
    assert_eq!(
        m.constraints[0],
        EntityConstraint::unique(["user_id", "product"]),
    );
    assert_eq!(
        m.constraints[1],
        EntityConstraint::check("user_id IS NOT NULL"),
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
    assert_eq!(&*f.name, "email");
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
    let m = basic_model();
    let names: Vec<&str> = m.field_names().collect();
    assert_eq!(names, vec!["id", "name", "email"]);
}

// ── 9. Entity::primary_keys ──────────────────────────────────────────

#[test]
fn primary_keys_filter() {
    let m = basic_model();
    let pks: Vec<_> = m.primary_keys().collect();
    assert_eq!(pks.len(), 1);
    assert_eq!(&*pks[0].name, "id");
    assert!(pks[0].primary_key);
}

// ── 10. Entity::non_pk_fields ────────────────────────────────────────

#[test]
fn non_pk_fields_filter() {
    let m = basic_model();
    let non_pk: Vec<_> = m.non_pk_fields().collect();
    assert_eq!(non_pk.len(), 2);
    assert_eq!(&*non_pk[0].name, "name");
    assert_eq!(&*non_pk[1].name, "email");
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
    assert_eq!(&*f.name, "col");
    assert_eq!(f.data_type, DataType::Int32);
    assert!(!f.primary_key);
    assert!(!f.nullable);
    assert!(!f.has_default);
    assert!(f.default_expr.is_none());
    assert!(!f.unique);
    assert!(f.references.is_none());
    assert!(f.check.is_none());
    assert!(f.comment.is_none());
    assert!(f.collation.is_none());
    assert!(f.generated.is_none());
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
    assert_eq!(f.default_expr.as_deref(), Some("'active'"));
    assert_eq!(f.check.as_deref(), Some("status IN ('active','inactive')"));
    assert_eq!(f.comment.as_deref(), Some("User account status"));
    assert_eq!(f.collation.as_deref(), Some("C"));
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
    assert!(f.default_expr.is_none());
}

#[test]
fn field_default_sets_expr() {
    let f = Field::new("ts", DataType::TimestampTz { precision: 6 }).default("NOW()");
    assert!(f.has_default);
    assert_eq!(f.default_expr.as_deref(), Some("NOW()"));
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
    assert_eq!(&*fk.table, "users");
    assert_eq!(&*fk.column, "id");
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
    assert_eq!(&*got.table, "orgs");
    assert_eq!(&*got.column, "org_id");
    assert_eq!(got.on_delete, FkAction::SetNull);
    assert_eq!(got.on_update, FkAction::Restrict);
}

// ── Field::generated_stored / generated_virtual ─────────────────────

#[test]
fn field_generated_stored() {
    let f = Field::new("total", DataType::Int32).generated_stored("price * qty");
    let (kind, expr) = f.generated.as_ref().unwrap();
    assert_eq!(*kind, GeneratedKind::Stored);
    assert_eq!(&**expr, "price * qty");
}

#[test]
fn field_generated_virtual() {
    let f = Field::new("full_name", DataType::Text).generated_virtual("first || ' ' || last");
    let (kind, expr) = f.generated.as_ref().unwrap();
    assert_eq!(*kind, GeneratedKind::Virtual);
    assert_eq!(&**expr, "first || ' ' || last");
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
    assert_eq!(&*fk.table, "users");
    assert_eq!(&*fk.column, "id");
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
    let c = EntityConstraint::unique(["a", "b"]);
    assert_eq!(c, EntityConstraint::unique(["a", "b"]));
}

#[test]
fn model_constraint_foreign_key() {
    let c = EntityConstraint::foreign_key(
        ["user_id"],
        "users",
        ["id"],
        FkAction::Cascade,
    );
    if let EntityConstraint::ForeignKey {
        columns,
        ref_table,
        ref_columns,
        on_delete,
    } = &c
    {
        let cols: Vec<&str> = columns.iter().map(|s| s.as_ref()).collect();
        let refs: Vec<&str> = ref_columns.iter().map(|s| s.as_ref()).collect();
        assert_eq!(cols, vec!["user_id"]);
        assert_eq!(&**ref_table, "users");
        assert_eq!(refs, vec!["id"]);
        assert_eq!(*on_delete, FkAction::Cascade);
    } else {
        panic!("expected ForeignKey variant");
    }
}

#[test]
fn model_constraint_check() {
    let c = EntityConstraint::check("age > 0");
    assert_eq!(c, EntityConstraint::check("age > 0"));
}

#[test]
fn model_constraint_primary_key() {
    let c = EntityConstraint::primary_key(["tenant_id", "user_id"]);
    assert_eq!(c, EntityConstraint::primary_key(["tenant_id", "user_id"]));
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

// ── Arc<str> ergonomics ─────────────────────────────────────────────

#[test]
fn entity_accepts_arc_str_input() {
    let name: Arc<str> = Arc::from("dynamic_table");
    let e = Entity::new(name.clone(), vec![]);
    assert_eq!(&*e.name, "dynamic_table");
    // Same Arc reused, no extra allocation.
    assert!(Arc::ptr_eq(&name, &e.name));
}

#[test]
fn entity_accepts_string_input() {
    let name = String::from("dynamic_table");
    let e = Entity::new(name, vec![]);
    assert_eq!(&*e.name, "dynamic_table");
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
