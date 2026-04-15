//! # dol-model — DOL Schema Language
//!
//! Model metadata — the universal schema definition.
//!
//! In DOL, a **Model** is the neutral term for any structured data shape:
//! - SQL: table
//! - Document store: collection
//! - Object store: bucket schema
//! - File system: typed resource
//!
//! A **Field** is a named property within a Model (SQL: column).
//!
//! A **FieldType** is the backend-agnostic logical type (SQL: column type).

#![deny(unsafe_code)]

pub mod constraint;
pub mod field;
pub mod field_type;

pub use constraint::{FkAction, ForeignKeyRef, GeneratedKind, ModelConstraint};
pub use field::Field;
pub use field_type::FieldType;

use constraint::ModelConstraint as Constraint;
use field::Field as F;

/// A model definition — the single source of truth for a data shape's schema.
///
/// Define as a `static` in your domain crate:
/// ```rust
/// use dol_model::{Model, Field, FieldType, FkAction, ModelConstraint};
///
/// pub static USERS: Model = Model::new("users", &[
///     Field::new("id", FieldType::Uuid).primary_key(),
///     Field::new("tenant_id", FieldType::Uuid),
///     Field::new("email", FieldType::Text),
///     Field::new("status", FieldType::Text).default("'active'"),
///     Field::new("created_at", FieldType::Timestamp).default("NOW()"),
/// ]).with_constraints(&[
///     ModelConstraint::Unique(&["tenant_id", "email"]),
/// ]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Model {
    pub name: &'static str,
    pub namespace: Option<&'static str>,
    pub fields: &'static [F],
    pub constraints: &'static [Constraint],
}

impl Model {
    pub const fn new(name: &'static str, fields: &'static [F]) -> Self {
        Self {
            name,
            namespace: None,
            fields,
            constraints: &[],
        }
    }

    pub const fn with_namespace(mut self, namespace: &'static str) -> Self {
        self.namespace = Some(namespace);
        self
    }

    pub const fn with_constraints(mut self, constraints: &'static [Constraint]) -> Self {
        self.constraints = constraints;
        self
    }

    /// Returns the fully-qualified model name (`namespace.name` or just `name`).
    pub fn qualified_name(&self) -> String {
        match self.namespace {
            Some(ns) => format!("{}.{}", ns, self.name),
            None => self.name.to_string(),
        }
    }

    /// Look up a field by name. Panics if not found (design-time error).
    pub fn field(&self, name: &str) -> &Field {
        self.fields
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("field '{}' not found in model '{}'", name, self.name))
    }

    /// Look up a field by name, returning `None` if not found.
    pub fn try_field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Returns an iterator over field names.
    pub fn field_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.fields.iter().map(|f| f.name)
    }

    /// Returns an iterator over primary-key fields.
    pub fn primary_keys(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter().filter(|f| f.primary_key)
    }

    /// Returns an iterator over non-primary-key fields.
    pub fn non_pk_fields(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter().filter(|f| !f.primary_key)
    }

    /// Comma-separated field list for SELECT or INSERT.
    pub fn field_list(&self) -> String {
        self.fields
            .iter()
            .map(|f| f.name)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper statics for Model tests ──────────────────────────────────

    static EMPTY_FIELDS: &[Field] = &[];

    static BASIC_FIELDS: &[Field] = &[
        Field::new("id", FieldType::Uuid).primary_key(),
        Field::new("name", FieldType::Text),
        Field::new("email", FieldType::Text).nullable(),
    ];

    static BASIC_MODEL: Model = Model::new("users", BASIC_FIELDS);

    static NS_MODEL: Model = Model::new("users", BASIC_FIELDS).with_namespace("public");

    static CONSTRAINED_MODEL: Model = Model::new(
        "orders",
        &[
            Field::new("id", FieldType::Serial).primary_key(),
            Field::new("user_id", FieldType::Uuid),
            Field::new("product", FieldType::Text),
        ],
    )
    .with_constraints(&[
        ModelConstraint::Unique(&["user_id", "product"]),
        ModelConstraint::Check("user_id IS NOT NULL"),
    ]);

    // ── 1. Model::new ───────────────────────────────────────────────────

    #[test]
    fn model_new_defaults() {
        let m = Model::new("items", EMPTY_FIELDS);
        assert_eq!(m.name, "items");
        assert_eq!(m.namespace, None);
        assert_eq!(m.fields.len(), 0);
        assert_eq!(m.constraints.len(), 0);
    }

    // ── 2. Model::with_namespace ────────────────────────────────────────

    #[test]
    fn model_with_namespace() {
        assert_eq!(NS_MODEL.namespace, Some("public"));
        assert_eq!(NS_MODEL.name, "users");
    }

    // ── 3. Model::with_constraints ──────────────────────────────────────

    #[test]
    fn model_with_constraints() {
        assert_eq!(CONSTRAINED_MODEL.constraints.len(), 2);
        assert_eq!(
            CONSTRAINED_MODEL.constraints[0],
            ModelConstraint::Unique(&["user_id", "product"]),
        );
        assert_eq!(
            CONSTRAINED_MODEL.constraints[1],
            ModelConstraint::Check("user_id IS NOT NULL"),
        );
    }

    // ── 4. Model::qualified_name ────────────────────────────────────────

    #[test]
    fn qualified_name_without_namespace() {
        assert_eq!(BASIC_MODEL.qualified_name(), "users");
    }

    #[test]
    fn qualified_name_with_namespace() {
        assert_eq!(NS_MODEL.qualified_name(), "public.users");
    }

    // ── 5 & 6. Model::field ─────────────────────────────────────────────

    #[test]
    fn field_lookup_success() {
        let f = BASIC_MODEL.field("email");
        assert_eq!(f.name, "email");
        assert!(f.nullable);
    }

    #[test]
    #[should_panic(expected = "field 'missing' not found in model 'users'")]
    fn field_lookup_panic_on_missing() {
        BASIC_MODEL.field("missing");
    }

    // ── 7. Model::try_field ─────────────────────────────────────────────

    #[test]
    fn try_field_some() {
        assert!(BASIC_MODEL.try_field("id").is_some());
    }

    #[test]
    fn try_field_none() {
        assert!(BASIC_MODEL.try_field("nonexistent").is_none());
    }

    // ── 8. Model::field_names ───────────────────────────────────────────

    #[test]
    fn field_names_iterator() {
        let names: Vec<_> = BASIC_MODEL.field_names().collect();
        assert_eq!(names, vec!["id", "name", "email"]);
    }

    // ── 9. Model::primary_keys ──────────────────────────────────────────

    #[test]
    fn primary_keys_filter() {
        let pks: Vec<_> = BASIC_MODEL.primary_keys().collect();
        assert_eq!(pks.len(), 1);
        assert_eq!(pks[0].name, "id");
        assert!(pks[0].primary_key);
    }

    // ── 10. Model::non_pk_fields ────────────────────────────────────────

    #[test]
    fn non_pk_fields_filter() {
        let non_pk: Vec<_> = BASIC_MODEL.non_pk_fields().collect();
        assert_eq!(non_pk.len(), 2);
        assert_eq!(non_pk[0].name, "name");
        assert_eq!(non_pk[1].name, "email");
    }

    // ── 11. Model::field_list ───────────────────────────────────────────

    #[test]
    fn field_list_comma_separated() {
        assert_eq!(BASIC_MODEL.field_list(), "id, name, email");
    }

    #[test]
    fn field_list_empty_model() {
        let m = Model::new("empty", EMPTY_FIELDS);
        assert_eq!(m.field_list(), "");
    }

    // ── 12. Field::new defaults ─────────────────────────────────────────

    #[test]
    fn field_new_defaults() {
        let f = Field::new("col", FieldType::Int);
        assert_eq!(f.name, "col");
        assert_eq!(f.field_type, FieldType::Int);
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
    }

    // ── 13. Field builder chain ─────────────────────────────────────────

    #[test]
    fn field_builder_chain() {
        let f = Field::new("status", FieldType::Text)
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
        let f = Field::new("bio", FieldType::Text).optional();
        assert!(f.nullable);
    }

    // ── 15. Field::required ─────────────────────────────────────────────

    #[test]
    fn field_required_is_noop() {
        let f = Field::new("bio", FieldType::Text).required();
        assert!(!f.nullable);
        // Ensure it returns the same field unchanged.
        assert_eq!(f, Field::new("bio", FieldType::Text));
    }

    // ── 16. Field::has_default (metadata only) ──────────────────────────

    #[test]
    fn field_has_default_metadata() {
        let f = Field::new("seq", FieldType::Int).has_default();
        assert!(f.has_default);
        assert_eq!(f.default_expr, None);
    }

    // ── Field::default sets both has_default and expr ───────────────────

    #[test]
    fn field_default_sets_expr() {
        let f = Field::new("ts", FieldType::Timestamp).default("NOW()");
        assert!(f.has_default);
        assert_eq!(f.default_expr, Some("NOW()"));
    }

    // ── 16. Field::references ───────────────────────────────────────────

    #[test]
    fn field_references_inline() {
        let f = Field::new("user_id", FieldType::Uuid).references(
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

    // ── 17. Field::references_full ──────────────────────────────────────

    #[test]
    fn field_references_full_from_builder() {
        let fk = ForeignKeyRef::new("orgs", "org_id")
            .on_delete(FkAction::SetNull)
            .on_update(FkAction::Restrict);
        let f = Field::new("org_id", FieldType::Uuid).references_full(fk);
        let got = f.references.unwrap();
        assert_eq!(got.table, "orgs");
        assert_eq!(got.column, "org_id");
        assert_eq!(got.on_delete, FkAction::SetNull);
        assert_eq!(got.on_update, FkAction::Restrict);
    }

    // ── 18. Field::generated_stored / generated_virtual ─────────────────

    #[test]
    fn field_generated_stored() {
        let f = Field::new("total", FieldType::Int).generated_stored("price * qty");
        assert_eq!(f.generated, Some((GeneratedKind::Stored, "price * qty")));
    }

    #[test]
    fn field_generated_virtual() {
        let f = Field::new("full_name", FieldType::Text).generated_virtual("first || ' ' || last");
        assert_eq!(
            f.generated,
            Some((GeneratedKind::Virtual, "first || ' ' || last")),
        );
    }

    // ── 19. Field::index ────────────────────────────────────────────────

    #[test]
    fn field_index_hint() {
        let f = Field::new("email", FieldType::Text).index();
        assert!(f.indexed);
    }

    // ── 20. FieldType Display — core variants ───────────────────────────

    #[test]
    fn field_type_display_core() {
        assert_eq!(FieldType::Text.to_string(), "TEXT");
        assert_eq!(FieldType::Int.to_string(), "INTEGER");
        assert_eq!(FieldType::SmallInt.to_string(), "SMALLINT");
        assert_eq!(FieldType::BigInt.to_string(), "BIGINT");
        assert_eq!(FieldType::Float.to_string(), "REAL");
        assert_eq!(FieldType::Double.to_string(), "DOUBLE PRECISION");
        assert_eq!(FieldType::Decimal.to_string(), "NUMERIC");
        assert_eq!(FieldType::Bool.to_string(), "BOOLEAN");
        assert_eq!(FieldType::Uuid.to_string(), "UUID");
        assert_eq!(FieldType::Bytes.to_string(), "BYTEA");
        assert_eq!(FieldType::Timestamp.to_string(), "TIMESTAMPTZ");
        assert_eq!(FieldType::Date.to_string(), "DATE");
        assert_eq!(FieldType::Time.to_string(), "TIME");
        assert_eq!(FieldType::Duration.to_string(), "INTERVAL");
        assert_eq!(FieldType::Json.to_string(), "JSONB");
        assert_eq!(FieldType::Inet.to_string(), "INET");
    }

    #[test]
    fn field_type_display_composite_and_storage() {
        assert_eq!(FieldType::Object.to_string(), "JSONB");
        assert_eq!(FieldType::TextArray.to_string(), "TEXT[]");
        assert_eq!(FieldType::Blob.to_string(), "BYTEA");
        assert_eq!(FieldType::Path.to_string(), "TEXT");
    }

    #[test]
    fn field_type_display_capability() {
        assert_eq!(FieldType::Url.to_string(), "TEXT");
        assert_eq!(FieldType::ResourceId.to_string(), "TEXT");
        assert_eq!(FieldType::Version.to_string(), "INTEGER");
        assert_eq!(FieldType::Etag.to_string(), "TEXT");
        assert_eq!(FieldType::Mime.to_string(), "TEXT");
    }

    #[test]
    fn field_type_display_serial() {
        assert_eq!(FieldType::Serial.to_string(), "SERIAL");
        assert_eq!(FieldType::BigSerial.to_string(), "BIGSERIAL");
    }

    // ── 21. FieldType with parameters ───────────────────────────────────

    #[test]
    fn field_type_char() {
        assert_eq!(FieldType::Char(10).to_string(), "CHAR(10)");
        assert_eq!(FieldType::Char(1).to_string(), "CHAR(1)");
    }

    #[test]
    fn field_type_varchar_bounded() {
        assert_eq!(FieldType::Varchar(Some(255)).to_string(), "VARCHAR(255)");
    }

    #[test]
    fn field_type_varchar_unbounded() {
        assert_eq!(FieldType::Varchar(None).to_string(), "VARCHAR");
    }

    #[test]
    fn field_type_custom() {
        assert_eq!(FieldType::Custom("CITEXT").to_string(), "CITEXT");
        assert_eq!(FieldType::Custom("MONEY").to_string(), "MONEY");
    }

    // ── 22. FkAction Display — all 5 variants ──────────────────────────

    #[test]
    fn fk_action_display() {
        assert_eq!(FkAction::NoAction.to_string(), "NO ACTION");
        assert_eq!(FkAction::Cascade.to_string(), "CASCADE");
        assert_eq!(FkAction::SetNull.to_string(), "SET NULL");
        assert_eq!(FkAction::Restrict.to_string(), "RESTRICT");
        assert_eq!(FkAction::SetDefault.to_string(), "SET DEFAULT");
    }

    // ── 23. GeneratedKind — Debug, Clone, Copy ─────────────────────────

    #[test]
    fn generated_kind_debug_clone_copy() {
        let stored = GeneratedKind::Stored;
        let copied = stored; // Copy
        assert_eq!(stored, copied);
        // Debug
        assert_eq!(format!("{:?}", GeneratedKind::Stored), "Stored");
        assert_eq!(format!("{:?}", GeneratedKind::Virtual), "Virtual");
    }

    // ── 24. ForeignKeyRef::new — defaults and builders ─────────────────

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

    // ── 25. ModelConstraint variants ────────────────────────────────────

    #[test]
    fn model_constraint_unique() {
        let c = ModelConstraint::Unique(&["a", "b"]);
        assert_eq!(c, ModelConstraint::Unique(&["a", "b"]));
    }

    #[test]
    fn model_constraint_foreign_key() {
        let c = ModelConstraint::ForeignKey {
            columns: &["user_id"],
            ref_table: "users",
            ref_columns: &["id"],
            on_delete: FkAction::Cascade,
        };
        if let ModelConstraint::ForeignKey {
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
        let c = ModelConstraint::Check("age > 0");
        assert_eq!(c, ModelConstraint::Check("age > 0"));
    }

    #[test]
    fn model_constraint_primary_key() {
        let c = ModelConstraint::PrimaryKey(&["tenant_id", "user_id"]);
        assert_eq!(c, ModelConstraint::PrimaryKey(&["tenant_id", "user_id"]));
    }

    // ── Model equality ─────────────────────────────────────────────────

    #[test]
    fn model_equality() {
        let a = Model::new("t", EMPTY_FIELDS);
        let b = Model::new("t", EMPTY_FIELDS);
        assert_eq!(a, b);
    }

    #[test]
    fn model_inequality() {
        let a = Model::new("t1", EMPTY_FIELDS);
        let b = Model::new("t2", EMPTY_FIELDS);
        assert_ne!(a, b);
    }
}
