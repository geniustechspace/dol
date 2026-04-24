//! # dol-schema — DOL Schema Language
//!
//! Entity metadata — the universal schema definition.
//!
//! In DOL, an **Entity** is the neutral term for any structured data shape:
//! - SQL: table
//! - Document store: collection
//! - Object store: bucket schema
//! - File system: typed resource
//!
//! A **Field** is a named property within an Entity.
//!
//! A **DataType** is the backend-agnostic logical type descriptor.

#![deny(unsafe_code)]

pub mod constraint;
pub mod definition;
pub mod field;
pub mod field_type;

pub use constraint::{EntityConstraint, FkAction, ForeignKeyRef, GeneratedKind};
pub use field::Field;
pub use field_type::DataType;

pub use definition::{
    AlterEntityBuilder, CreateFromMeta, DefineEntityBuilder, DefineIndexBuilder, DefineTypeBuilder,
    DropEntityBuilder, DropIndexBuilder, DropTypeBuilder, EntityDefineExt,
};

use std::sync::Arc;

use constraint::EntityConstraint as Constraint;

/// A model definition — the single source of truth for a data shape's schema.
///
/// # Example
///
/// ```rust
/// use dol_schema::{Entity, Field, DataType, FkAction, EntityConstraint};
///
/// let users = Entity::new("users", vec![
///     Field::new("id", DataType::Uuid).primary_key(),
///     Field::new("tenant_id", DataType::Uuid),
///     Field::new("email", DataType::Text),
///     Field::new("status", DataType::Text).default("'active'"),
///     Field::new("seq", DataType::Int32).auto_increment(),
/// ]).with_constraints(vec![
///     EntityConstraint::unique(["tenant_id", "email"]),
/// ]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Entity {
    pub name: Arc<str>,
    pub namespace: Option<Arc<str>>,
    pub fields: Vec<Field>,
    pub constraints: Vec<Constraint>,
}

impl Entity {
    pub fn new(name: impl Into<Arc<str>>, fields: Vec<Field>) -> Self {
        Self {
            name: name.into(),
            namespace: None,
            fields,
            constraints: Vec::new(),
        }
    }

    pub fn with_namespace(mut self, namespace: impl Into<Arc<str>>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    pub fn with_constraints(mut self, constraints: Vec<Constraint>) -> Self {
        self.constraints = constraints;
        self
    }

    /// Returns the fully-qualified model name (`namespace.name` or just `name`).
    pub fn qualified_name(&self) -> String {
        match &self.namespace {
            Some(ns) => format!("{}.{}", ns, self.name),
            None => self.name.to_string(),
        }
    }

    /// Look up a field by name. Panics if not found (design-time error).
    pub fn field(&self, name: &str) -> &Field {
        self.fields
            .iter()
            .find(|f| &*f.name == name)
            .unwrap_or_else(|| panic!("field '{}' not found in entity '{}'", name, self.name))
    }

    /// Look up a field by name, returning `None` if not found.
    pub fn try_field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| &*f.name == name)
    }

    /// Returns an iterator over field names.
    pub fn field_names(&self) -> impl Iterator<Item = &str> + '_ {
        self.fields.iter().map(|f| &*f.name)
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
            .map(|f| f.name.as_ref())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[cfg(test)]
mod tests;
