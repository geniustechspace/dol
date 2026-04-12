//! Schema diff engine — compares models and auto-generates migration steps.
//!
//! This module powers the "self-discovery" aspect of the migration system:
//! given an old and new [`Model`] definition, it computes the minimal set of
//! [`AlterAction`] operations needed to bring the schema up to date.
//!
//! # Usage
//!
//! ```rust
//! use dol_core::model::{Model, Field, FieldType};
//! use dol_migration::schema_diff::{diff_models, diff_to_steps, ModelSnapshot};
//!
//! static OLD: Model = Model::new("users", &[
//!     Field::new("id", FieldType::Uuid).primary_key(),
//!     Field::new("email", FieldType::Text).unique(),
//! ]);
//!
//! static NEW: Model = Model::new("users", &[
//!     Field::new("id", FieldType::Uuid).primary_key(),
//!     Field::new("email", FieldType::Text).unique(),
//!     Field::new("name", FieldType::Text).nullable(),
//! ]);
//!
//! let old_snap = ModelSnapshot::from_model(&OLD);
//! let new_snap = ModelSnapshot::from_model(&NEW);
//! let actions = diff_models(&old_snap, &new_snap);
//!
//! assert_eq!(actions.len(), 1); // AddField("name")
//!
//! let steps = diff_to_steps("users", &old_snap, &new_snap);
//! assert_eq!(steps.len(), 1);
//! ```

use dol_core::ir::AlterAction;
use dol_core::ir::definition::{FieldDef, OwnedForeignKeyRef};
use dol_core::model::{Field, FieldType, Model};

use super::MigrationStep;

// ===========================================================================
// ModelSnapshot — a cheaply-clonable snapshot of a Model's schema
// ===========================================================================

/// A snapshot of a model's field-level schema for diffing.
///
/// This is intentionally lightweight — it captures only the field metadata
/// that affects schema DDL, not runtime query behaviour.
#[derive(Debug, Clone)]
pub struct ModelSnapshot {
    /// Model name.
    pub name: String,
    /// Fields in order.
    pub fields: Vec<FieldSnapshot>,
}

/// A snapshot of a single field's schema.
#[derive(Debug, Clone)]
pub struct FieldSnapshot {
    pub name: String,
    pub field_type: FieldType,
    pub primary_key: bool,
    pub nullable: bool,
    pub has_default: bool,
    pub default_expr: Option<String>,
    pub unique: bool,
    pub indexed: bool,
}

impl ModelSnapshot {
    /// Create a snapshot from a static `Model` definition.
    pub fn from_model(model: &Model) -> Self {
        Self {
            name: model.name.to_string(),
            fields: model.fields.iter().map(FieldSnapshot::from_field).collect(),
        }
    }

    /// Create a snapshot from a list of `FieldDef` (dynamic definitions).
    pub fn from_field_defs(name: &str, fields: &[FieldDef]) -> Self {
        Self {
            name: name.to_string(),
            fields: fields.iter().map(FieldSnapshot::from_field_def).collect(),
        }
    }
}

impl FieldSnapshot {
    /// Create from a static `Field`.
    pub fn from_field(field: &Field) -> Self {
        Self {
            name: field.name.to_string(),
            field_type: field.field_type,
            primary_key: field.primary_key,
            nullable: field.nullable,
            has_default: field.has_default,
            default_expr: field.default_expr.map(|s| s.to_string()),
            unique: field.unique,
            indexed: field.indexed,
        }
    }

    /// Create from an owned `FieldDef`.
    pub fn from_field_def(def: &FieldDef) -> Self {
        Self {
            name: def.name.clone(),
            field_type: def.field_type,
            primary_key: def.primary_key,
            nullable: def.nullable,
            has_default: def.default_expr.is_some(),
            default_expr: def.default_expr.clone(),
            unique: def.unique,
            indexed: def.indexed,
        }
    }

    /// Convert to a `FieldDef` for use in migration steps.
    fn to_field_def(&self) -> FieldDef {
        let mut def = FieldDef::new(&self.name, self.field_type);
        if self.primary_key {
            def = def.primary_key();
        }
        if self.nullable {
            def = def.nullable();
        }
        if let Some(expr) = &self.default_expr {
            def = def.default(expr);
        }
        if self.unique {
            def = def.unique();
        }
        if self.indexed {
            def = def.index();
        }
        def
    }
}

// ===========================================================================
// Diff engine
// ===========================================================================

/// Compute the list of `AlterAction` needed to transform `old` into `new`.
///
/// The diff algorithm:
/// 1. Fields in `new` but not in `old` → `AddField`
/// 2. Fields in `old` but not in `new` → `DropField`
/// 3. Fields in both with different types → `AlterFieldType`
/// 4. Fields with changed nullability → `SetFieldNotNull` / `DropFieldNotNull`
/// 5. Fields with changed defaults → `SetFieldDefault` / `DropFieldDefault`
pub fn diff_models(old: &ModelSnapshot, new: &ModelSnapshot) -> Vec<AlterAction> {
    let mut actions = Vec::new();

    let old_fields: std::collections::HashMap<&str, &FieldSnapshot> =
        old.fields.iter().map(|f| (f.name.as_str(), f)).collect();
    let new_fields: std::collections::HashMap<&str, &FieldSnapshot> =
        new.fields.iter().map(|f| (f.name.as_str(), f)).collect();

    // 1. Added fields (in new but not in old) — preserve order from `new`
    for new_field in &new.fields {
        if !old_fields.contains_key(new_field.name.as_str()) {
            actions.push(AlterAction::AddField(new_field.to_field_def()));
        }
    }

    // 2. Dropped fields (in old but not in new)
    for old_field in &old.fields {
        if !new_fields.contains_key(old_field.name.as_str()) {
            actions.push(AlterAction::DropField(old_field.name.clone()));
        }
    }

    // 3-5. Changed fields (in both)
    for new_field in &new.fields {
        if let Some(old_field) = old_fields.get(new_field.name.as_str()) {
            // Type change
            if old_field.field_type != new_field.field_type {
                actions.push(AlterAction::AlterFieldType {
                    name: new_field.name.clone(),
                    new_type: new_field.field_type,
                });
            }

            // Nullability change
            if old_field.nullable && !new_field.nullable {
                actions.push(AlterAction::SetFieldNotNull(new_field.name.clone()));
            } else if !old_field.nullable && new_field.nullable {
                actions.push(AlterAction::DropFieldNotNull(new_field.name.clone()));
            }

            // Default change
            match (&old_field.default_expr, &new_field.default_expr) {
                (None, Some(expr)) => {
                    actions.push(AlterAction::SetFieldDefault {
                        name: new_field.name.clone(),
                        expr: expr.clone(),
                    });
                }
                (Some(_), None) => {
                    actions.push(AlterAction::DropFieldDefault(new_field.name.clone()));
                }
                (Some(old_expr), Some(new_expr)) if old_expr != new_expr => {
                    actions.push(AlterAction::SetFieldDefault {
                        name: new_field.name.clone(),
                        expr: new_expr.clone(),
                    });
                }
                _ => {}
            }
        }
    }

    actions
}

/// Generate [`MigrationStep`]s from a model diff.
///
/// If both models are `None`, returns no steps.
/// If `old` is `None`, generates a `DefineModel` step (CREATE TABLE).
/// If `new` is `None`, generates a `DropModel` step (DROP TABLE).
/// Otherwise, computes the diff and generates `AlterModel` steps.
pub fn diff_to_steps(
    model_name: &str,
    old: &ModelSnapshot,
    new: &ModelSnapshot,
) -> Vec<MigrationStep> {
    let actions = diff_models(old, new);
    if actions.is_empty() {
        return Vec::new();
    }
    vec![MigrationStep::alter_model(model_name, actions)]
}

/// Generate a forward migration step for a brand new model.
///
/// Converts a [`Model`] to a `DefineModel` IR step with all its fields.
pub fn create_model_step(model: &Model) -> MigrationStep {
    use dol_core::builder::DefineModelBuilder;

    let mut builder = DefineModelBuilder::new(model.name);
    if let Some(ns) = model.namespace {
        builder = builder.namespace(ns);
    }
    for field in model.fields {
        builder = builder.field(field_to_field_def(field));
    }
    builder = builder.if_not_exists();
    MigrationStep::define_model(builder.build())
}

/// Generate a backward migration step (DROP TABLE) for a model.
pub fn drop_model_step(model: &Model) -> MigrationStep {
    MigrationStep::drop_model(model.name)
}

/// Convert a static `Field` to an owned `FieldDef`.
pub fn field_to_field_def(field: &Field) -> FieldDef {
    let mut def = FieldDef::new(field.name, field.field_type);
    if field.primary_key {
        def = def.primary_key();
    }
    if field.nullable {
        def = def.nullable();
    }
    if let Some(expr) = field.default_expr {
        def = def.default(expr);
    }
    if field.unique {
        def = def.unique();
    }
    if let Some(fk) = &field.references {
        def = def.references(
            OwnedForeignKeyRef::new(fk.table, fk.column)
                .on_delete(fk.on_delete)
                .on_update(fk.on_update),
        );
    }
    if let Some(expr) = field.check {
        def = def.check(expr);
    }
    if let Some(text) = field.comment {
        def = def.comment(text);
    }
    if let Some(collation) = field.collation {
        def = def.collation(collation);
    }
    if field.indexed {
        def = def.index();
    }
    if let Some((kind, expr)) = &field.generated {
        def = match kind {
            dol_core::model::constraint::GeneratedKind::Stored => def.generated_stored(expr),
            dol_core::model::constraint::GeneratedKind::Virtual => def.generated_virtual(expr),
        };
    }
    def
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    static OLD_MODEL: Model = Model::new(
        "users",
        &[
            Field::new("id", FieldType::Uuid).primary_key(),
            Field::new("email", FieldType::Text).unique(),
            Field::new("status", FieldType::Text).default("'active'"),
        ],
    );

    static NEW_MODEL: Model = Model::new(
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
        let old = ModelSnapshot::from_model(&OLD_MODEL);
        let new = ModelSnapshot::from_model(&NEW_MODEL);
        let actions = diff_models(&old, &new);
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], AlterAction::AddField(f) if f.name == "name"));
    }

    #[test]
    fn diff_drop_field() {
        let old = ModelSnapshot::from_model(&NEW_MODEL); // has "name"
        let new = ModelSnapshot::from_model(&OLD_MODEL); // no "name"
        let actions = diff_models(&old, &new);
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], AlterAction::DropField(name) if name == "name"));
    }

    #[test]
    fn diff_change_type() {
        static V1: Model = Model::new(
            "t",
            &[
                Field::new("id", FieldType::Uuid).primary_key(),
                Field::new("count", FieldType::Int),
            ],
        );
        static V2: Model = Model::new(
            "t",
            &[
                Field::new("id", FieldType::Uuid).primary_key(),
                Field::new("count", FieldType::BigInt),
            ],
        );

        let actions = diff_models(
            &ModelSnapshot::from_model(&V1),
            &ModelSnapshot::from_model(&V2),
        );
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            &actions[0],
            AlterAction::AlterFieldType { name, new_type } if name == "count" && *new_type == FieldType::BigInt
        ));
    }

    #[test]
    fn diff_change_nullability() {
        static V1: Model = Model::new("t", &[Field::new("name", FieldType::Text)]);
        static V2: Model = Model::new("t", &[Field::new("name", FieldType::Text).nullable()]);

        let actions = diff_models(
            &ModelSnapshot::from_model(&V1),
            &ModelSnapshot::from_model(&V2),
        );
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], AlterAction::DropFieldNotNull(n) if n == "name"));
    }

    #[test]
    fn diff_change_default() {
        static V1: Model = Model::new("t", &[Field::new("status", FieldType::Text)]);
        static V2: Model = Model::new(
            "t",
            &[Field::new("status", FieldType::Text).default("'new'")],
        );

        let actions = diff_models(
            &ModelSnapshot::from_model(&V1),
            &ModelSnapshot::from_model(&V2),
        );
        assert_eq!(actions.len(), 1);
        assert!(
            matches!(&actions[0], AlterAction::SetFieldDefault { name, expr } if name == "status" && expr == "'new'")
        );
    }

    #[test]
    fn diff_drop_default() {
        static V1: Model = Model::new(
            "t",
            &[Field::new("status", FieldType::Text).default("'old'")],
        );
        static V2: Model = Model::new("t", &[Field::new("status", FieldType::Text)]);

        let actions = diff_models(
            &ModelSnapshot::from_model(&V1),
            &ModelSnapshot::from_model(&V2),
        );
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], AlterAction::DropFieldDefault(n) if n == "status"));
    }

    #[test]
    fn diff_no_changes() {
        let snap = ModelSnapshot::from_model(&OLD_MODEL);
        let actions = diff_models(&snap, &snap.clone());
        assert!(actions.is_empty());
    }

    #[test]
    fn diff_multiple_changes() {
        static V1: Model = Model::new(
            "items",
            &[
                Field::new("id", FieldType::Uuid).primary_key(),
                Field::new("name", FieldType::Text),
                Field::new("removed_field", FieldType::Text),
            ],
        );
        static V2: Model = Model::new(
            "items",
            &[
                Field::new("id", FieldType::Uuid).primary_key(),
                Field::new("name", FieldType::Varchar(Some(255))),
                Field::new("added_field", FieldType::Int).nullable(),
            ],
        );

        let actions = diff_models(
            &ModelSnapshot::from_model(&V1),
            &ModelSnapshot::from_model(&V2),
        );
        // AddField(added_field), DropField(removed_field), AlterFieldType(name)
        assert_eq!(actions.len(), 3);
    }

    #[test]
    fn diff_to_steps_produces_alter() {
        let old = ModelSnapshot::from_model(&OLD_MODEL);
        let new = ModelSnapshot::from_model(&NEW_MODEL);
        let steps = diff_to_steps("users", &old, &new);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].kind(), "sql");
    }

    #[test]
    fn diff_to_steps_empty_when_no_changes() {
        let snap = ModelSnapshot::from_model(&OLD_MODEL);
        let steps = diff_to_steps("users", &snap, &snap.clone());
        assert!(steps.is_empty());
    }

    #[test]
    fn create_model_step_works() {
        let step = create_model_step(&OLD_MODEL);
        assert_eq!(step.kind(), "sql");
    }

    #[test]
    fn drop_model_step_works() {
        let step = drop_model_step(&OLD_MODEL);
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
        let snap = ModelSnapshot::from_field_defs("test", &fields);
        assert_eq!(snap.name, "test");
        assert_eq!(snap.fields.len(), 2);
        assert!(snap.fields[0].primary_key);
    }
}
