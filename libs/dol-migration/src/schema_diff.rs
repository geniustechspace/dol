//! Schema diff engine — compares models and auto-generates migration steps.
//!
//! This module powers the "self-discovery" aspect of the migration system:
//! given an old and new [`Model`] definition, it computes the minimal set of
//! [`AlterAction`] operations needed to bring the schema up to date.
//!
//! # Usage
//!
//! ```rust
//! use dol_entity::{Entity, Field, DataType};
//! use dol_migration::schema_diff::{diff_entities, diff_to_steps, EntitySnapshot};
//!
//! let old = Entity::new("users", vec![
//!     Field::new("id", DataType::Uuid).primary_key(),
//!     Field::new("email", DataType::Text).unique(),
//! ]);
//!
//! let new = Entity::new("users", vec![
//!     Field::new("id", DataType::Uuid).primary_key(),
//!     Field::new("email", DataType::Text).unique(),
//!     Field::new("name", DataType::Text).nullable(),
//! ]);
//!
//! let old_snap = EntitySnapshot::from_entity(&old);
//! let new_snap = EntitySnapshot::from_entity(&new);
//! let actions = diff_entities(&old_snap, &new_snap);
//!
//! assert_eq!(actions.len(), 1); // AddField("name")
//!
//! let steps = diff_to_steps("users", &old_snap, &new_snap);
//! assert_eq!(steps.len(), 1);
//! ```

use dol_core::ir::AlterAction;
use dol_core::ir::definition::{FieldDef, OwnedForeignKeyRef};
use dol_entity::{Entity, Field, DataType};

use super::MigrationStep;

// ===========================================================================
// EntitySnapshot — a cheaply-clonable snapshot of a Model's schema
// ===========================================================================

/// A snapshot of a model's field-level schema for diffing.
///
/// This is intentionally lightweight — it captures only the field metadata
/// that affects schema DDL, not runtime query behaviour.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntitySnapshot {
    /// Model name.
    pub name: String,
    /// Fields in order.
    pub fields: Vec<FieldSnapshot>,
}

/// A snapshot of a single field's schema.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FieldSnapshot {
    pub name: String,
    pub data_type: DataType,
    pub primary_key: bool,
    pub nullable: bool,
    pub has_default: bool,
    pub default_expr: Option<String>,
    pub unique: bool,
    pub indexed: bool,
}

impl EntitySnapshot {
    /// Create a snapshot from a static `Model` definition.
    pub fn from_entity(model: &Entity) -> Self {
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
            data_type: field.data_type.clone(),
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
            data_type: def.data_type.clone(),
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
        let mut def = FieldDef::new(&self.name, self.data_type.clone());
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
pub fn diff_entities(old: &EntitySnapshot, new: &EntitySnapshot) -> Vec<AlterAction> {
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
            if old_field.data_type != new_field.data_type {
                actions.push(AlterAction::AlterFieldType {
                    name: new_field.name.clone(),
                    new_type: new_field.data_type.clone(),
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
    old: &EntitySnapshot,
    new: &EntitySnapshot,
) -> Vec<MigrationStep> {
    let actions = diff_entities(old, new);
    if actions.is_empty() {
        return Vec::new();
    }
    vec![MigrationStep::alter_entity(model_name, actions)]
}

/// Generate a forward migration step for a brand new model.
///
/// Converts a [`Model`] to a `DefineModel` IR step with all its fields.
pub fn create_entity_step(model: &Entity) -> MigrationStep {
    use dol_entity::DefineEntityBuilder;

    let mut builder = DefineEntityBuilder::new(model.name);
    if let Some(ns) = model.namespace {
        builder = builder.namespace(ns);
    }
    for field in &model.fields {
        builder = builder.field(field_to_field_def(field));
    }
    builder = builder.if_not_exists();
    MigrationStep::define_entity(builder.build())
}

/// Generate a backward migration step (DROP TABLE) for a model.
pub fn drop_entity_step(model: &Entity) -> MigrationStep {
    MigrationStep::drop_entity(model.name)
}

/// Convert a static `Field` to an owned `FieldDef`.
pub fn field_to_field_def(field: &Field) -> FieldDef {
    let mut def = FieldDef::new(field.name, field.data_type.clone());
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
            dol_entity::constraint::GeneratedKind::Stored => def.generated_stored(expr),
            dol_entity::constraint::GeneratedKind::Virtual => def.generated_virtual(expr),
        };
    }
    def
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[path = "schema_diff_tests.rs"]
mod tests;
