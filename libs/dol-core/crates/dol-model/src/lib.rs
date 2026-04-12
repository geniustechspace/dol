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
