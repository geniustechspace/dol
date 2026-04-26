use std::sync::Arc;

use super::*;

// ── Helpers ─────────────────────────────────────────────────────────

fn basic_fields() -> Vec<Field> {
    vec![
        Field::new("id", DataType::Uuid).identity(),
        Field::new("name", DataType::unbounded_string()),
        Field::new("email", DataType::unbounded_string()).nullable(),
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
            Field::new("id", DataType::Int32).identity().auto_assign(),
            Field::new("user_id", DataType::Uuid),
            Field::new("product", DataType::unbounded_string()),
        ],
    )
    .with_constraints(vec![
        EntityConstraint::unique(["user_id", "product"]),
        EntityConstraint::invariant("user_id IS NOT NULL"),
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
        EntityConstraint::invariant("user_id IS NOT NULL"),
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
#[allow(deprecated)] // exercising the deprecated panicking accessor
fn field_lookup_success() {
    let m = basic_model();
    let f = m.field("email");
    assert_eq!(&*f.name, "email");
    assert!(f.nullable);
}

#[test]
#[should_panic(expected = "field 'missing' not found in entity 'users'")]
#[allow(deprecated)] // exercising the deprecated panicking accessor
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

// ── 9. Entity::identity_fields ──────────────────────────────────────────

#[test]
fn primary_keys_filter() {
    let m = basic_model();
    let pks: Vec<_> = m.identity_fields().collect();
    assert_eq!(pks.len(), 1);
    assert_eq!(&*pks[0].name, "id");
    assert!(pks[0].identity);
}

// ── 10. Entity::non_identity_fields ────────────────────────────────────────

#[test]
fn non_identity_fields_filter() {
    let m = basic_model();
    let non_identity: Vec<_> = m.non_identity_fields().collect();
    assert_eq!(non_identity.len(), 2);
    assert_eq!(&*non_identity[0].name, "name");
    assert_eq!(&*non_identity[1].name, "email");
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
    assert!(!f.identity);
    assert!(!f.nullable);
    assert!(!f.has_default);
    assert!(f.default_expr.is_none());
    assert!(!f.unique);
    assert!(f.references.is_none());
    assert!(f.check.is_none());
    assert!(f.comment.is_none());
    assert!(f.collation.is_none());
    assert!(f.generated.is_none());
    assert!(!f.lookup);
    assert!(!f.auto_assign);
}

// ── 13. Field builder chain ─────────────────────────────────────────

#[test]
fn field_builder_chain() {
    let f = Field::new("status", DataType::unbounded_string())
        .identity()
        .nullable()
        .unique()
        .default("'active'")
        .check("status IN ('active','inactive')")
        .comment("User account status")
        .collation("C");

    assert!(f.identity);
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
    let f = Field::new("bio", DataType::unbounded_string()).optional();
    assert!(f.nullable);
}

// ── 15. Field::required ─────────────────────────────────────────────

#[test]
fn field_required_is_noop() {
    let f = Field::new("bio", DataType::unbounded_string()).required();
    assert!(!f.nullable);
    assert_eq!(f, Field::new("bio", DataType::unbounded_string()));
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
    let f = Field::new("ts", DataType::OffsetDateTime { precision: 6 }).default("NOW()");
    assert!(f.has_default);
    assert_eq!(f.default_expr.as_deref(), Some("NOW()"));
}

// ── Field::references ───────────────────────────────────────────────

#[test]
fn field_references_inline() {
    let f = Field::new("user_id", DataType::Uuid).references(
        "users",
        "id",
        RefAction::Cascade,
        RefAction::Forbid,
    );
    let fk = f.references.unwrap();
    assert_eq!(&*fk.entity, "users");
    assert_eq!(&*fk.field, "id");
    assert_eq!(fk.on_delete, RefAction::Cascade);
    assert_eq!(fk.on_update, RefAction::Forbid);
}

#[test]
fn field_references_full_from_builder() {
    let fk = RelationRef::new("orgs", "org_id")
        .on_delete(RefAction::Detach)
        .on_update(RefAction::Reject);
    let f = Field::new("org_id", DataType::Uuid).references_full(fk);
    let got = f.references.unwrap();
    assert_eq!(&*got.entity, "orgs");
    assert_eq!(&*got.field, "org_id");
    assert_eq!(got.on_delete, RefAction::Detach);
    assert_eq!(got.on_update, RefAction::Reject);
}

// ── Field::generated_stored / generated_virtual ─────────────────────

#[test]
fn field_generated_stored() {
    let f = Field::new("total", DataType::Int32).generated_stored("price * qty");
    let (kind, expr) = f.generated.as_ref().unwrap();
    assert_eq!(*kind, ComputedKind::Materialized);
    assert_eq!(&**expr, "price * qty");
}

#[test]
fn field_generated_virtual() {
    let f = Field::new("full_name", DataType::unbounded_string())
        .generated_virtual("first || ' ' || last");
    let (kind, expr) = f.generated.as_ref().unwrap();
    assert_eq!(*kind, ComputedKind::OnDemand);
    assert_eq!(&**expr, "first || ' ' || last");
}

// ── Field::index ────────────────────────────────────────────────────

#[test]
fn field_index_hint() {
    let f = Field::new("email", DataType::unbounded_string()).lookup();
    assert!(f.lookup);
}

// ── Field::auto_assign ───────────────────────────────────────────

#[test]
fn field_auto_increment() {
    let f = Field::new("id", DataType::Int32).auto_assign().identity();
    assert!(f.auto_assign);
    assert!(f.identity);
    assert_eq!(f.data_type, DataType::Int32);
}

#[test]
fn field_auto_increment_bigint() {
    let f = Field::new("id", DataType::Int64).auto_assign();
    assert!(f.auto_assign);
    assert_eq!(f.data_type, DataType::Int64);
}

// ── ComputedKind ───────────────────────────────────────────────────

#[test]
fn generated_kind_debug_clone_copy() {
    let stored = ComputedKind::Materialized;
    let copied = stored; // Copy
    assert_eq!(stored, copied);
    assert_eq!(format!("{:?}", ComputedKind::Materialized), "Materialized");
    assert_eq!(format!("{:?}", ComputedKind::OnDemand), "OnDemand");
}

// ── RelationRef ───────────────────────────────────────────────────

#[test]
fn foreign_key_ref_defaults() {
    let fk = RelationRef::new("users", "id");
    assert_eq!(&*fk.entity, "users");
    assert_eq!(&*fk.field, "id");
    assert_eq!(fk.on_delete, RefAction::Forbid);
    assert_eq!(fk.on_update, RefAction::Forbid);
}

#[test]
fn foreign_key_ref_builder() {
    let fk = RelationRef::new("users", "id")
        .on_delete(RefAction::Cascade)
        .on_update(RefAction::UseDefault);
    assert_eq!(fk.on_delete, RefAction::Cascade);
    assert_eq!(fk.on_update, RefAction::UseDefault);
}

// ── EntityConstraint variants ───────────────────────────────────────

#[test]
fn model_constraint_unique() {
    let c = EntityConstraint::unique(["a", "b"]);
    assert_eq!(c, EntityConstraint::unique(["a", "b"]));
}

#[test]
fn model_constraint_foreign_key() {
    let c = EntityConstraint::relation(["user_id"], "users", ["id"], RefAction::Cascade);
    if let EntityConstraint::Relation {
        fields,
        ref_entity,
        ref_fields,
        on_delete,
    } = &c
    {
        let cols: Vec<&str> = fields.iter().map(|s| s.as_ref()).collect();
        let refs: Vec<&str> = ref_fields.iter().map(|s| s.as_ref()).collect();
        assert_eq!(cols, vec!["user_id"]);
        assert_eq!(&**ref_entity, "users");
        assert_eq!(refs, vec!["id"]);
        assert_eq!(*on_delete, RefAction::Cascade);
    } else {
        panic!("expected Relation variant");
    }
}

#[test]
fn model_constraint_check() {
    let c = EntityConstraint::invariant("age > 0");
    assert_eq!(c, EntityConstraint::invariant("age > 0"));
}

#[test]
fn model_constraint_primary_key() {
    let c = EntityConstraint::identity(["tenant_id", "user_id"]);
    assert_eq!(c, EntityConstraint::identity(["tenant_id", "user_id"]));
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
    let _ = DataType::unbounded_string();
    let _ = DataType::Uuid;
    let _ = DataType::Int32;
    let _ = DataType::varying_string(255);
    let _ = DataType::Json;
    let _ = DataType::OffsetDateTime { precision: 6 };
    let _ = DataType::Array(Box::new(DataType::unbounded_string()));
}
