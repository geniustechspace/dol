//! Field definition within an Entity.
//!
//! In DOL, a **Field** is a named property with a type, optional constraints,
//! and an optional default expression. The DOL philosophy:
//! - Fields are **required** (NOT NULL) by default
//! - Use `.optional()` or `.nullable()` to make them nullable
//! - Use `.required()` as a no-op self-documenting marker

use std::sync::Arc;

use super::constraint::{RefAction, RelationRef, ComputedKind};
pub use dol_types::DataType;

/// A field definition within an entity.
///
/// Fields support the full range of constraints and references:
/// - `identity()` — marks as part of the primary key
/// - `nullable()` / `optional()` — allows NULL values (NOT NULL by default)
/// - `unique()` — adds a UNIQUE constraint
/// - `default(expr)` — sets a DEFAULT expression rendered in DDL
/// - `references(entity, field, on_delete, on_update)` — inline foreign key
/// - `check(expr)` — inline CHECK constraint
/// - `index()` — hints that this field should be lookup
/// - `collation(name)` — overrides the collation for this field
/// - `generated_stored(expr)` / `generated_virtual(expr)` — computed fields
/// - `auto_assign()` — marks as auto-incrementing (replaces Serial/BigSerial)
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Field {
    pub name: Arc<str>,
    pub data_type: DataType,
    pub identity: bool,
    pub nullable: bool,
    pub has_default: bool,
    pub default_expr: Option<Arc<str>>,
    pub unique: bool,
    pub references: Option<RelationRef>,
    /// Inline CHECK constraint expression.
    pub check: Option<Arc<str>>,
    /// Human-readable description / comment.
    pub comment: Option<Arc<str>>,
    /// Collation override (e.g. `"C"`, `"en_US.UTF-8"`).
    pub collation: Option<Arc<str>>,
    /// Generated (computed) field: `(kind, expression)`.
    pub generated: Option<(ComputedKind, Arc<str>)>,
    /// Hint that this field should be lookup (for schema generation tooling).
    pub lookup: bool,
    /// Auto-incrementing field (replaces the old Serial/BigSerial types).
    pub auto_assign: bool,
}

impl Field {
    pub fn new(name: impl Into<Arc<str>>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            identity: false,
            nullable: false,
            has_default: false,
            default_expr: None,
            unique: false,
            references: None,
            check: None,
            comment: None,
            collation: None,
            generated: None,
            lookup: false,
            auto_assign: false,
        }
    }

    pub fn identity(mut self) -> Self {
        self.identity = true;
        self
    }

    /// Mark field as nullable (optional in DOL terminology).
    pub fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }

    /// Mark the field as optional. Equivalent to [`nullable`](Self::nullable).
    pub fn optional(self) -> Self {
        self.nullable()
    }

    /// No-op self-documenting marker — fields are required by default.
    pub fn required(self) -> Self {
        self
    }

    /// Mark field as having a server-side default (metadata only, no DDL rendering).
    pub fn has_default(mut self) -> Self {
        self.has_default = true;
        self
    }

    /// Set a DEFAULT expression that will be rendered in DDL.
    pub fn default(mut self, expr: impl Into<Arc<str>>) -> Self {
        self.has_default = true;
        self.default_expr = Some(expr.into());
        self
    }

    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    /// Add an inline relation to another entity, with on-delete/on-update
    /// actions.
    pub fn references(
        mut self,
        entity: impl Into<Arc<str>>,
        field: impl Into<Arc<str>>,
        on_delete: RefAction,
        on_update: RefAction,
    ) -> Self {
        self.references = Some(RelationRef {
            entity: entity.into(),
            field: field.into(),
            on_delete,
            on_update,
        });
        self
    }

    /// Attach a fully-configured relation reference.
    pub fn references_full(mut self, fk: RelationRef) -> Self {
        self.references = Some(fk);
        self
    }

    /// Add an inline CHECK constraint expression.
    pub fn check(mut self, expr: impl Into<Arc<str>>) -> Self {
        self.check = Some(expr.into());
        self
    }

    /// Set a human-readable comment / description for this field.
    pub fn comment(mut self, text: impl Into<Arc<str>>) -> Self {
        self.comment = Some(text.into());
        self
    }

    /// Override the collation for this field.
    pub fn collation(mut self, collation: impl Into<Arc<str>>) -> Self {
        self.collation = Some(collation.into());
        self
    }

    /// Hint that this field should be lookup-optimised.
    pub fn lookup(mut self) -> Self {
        self.lookup = true;
        self
    }

    /// Mark as a stored generated (computed) field.
    pub fn generated_stored(mut self, expr: impl Into<Arc<str>>) -> Self {
        self.generated = Some((ComputedKind::Materialized, expr.into()));
        self
    }

    /// Mark as a virtual generated (computed) field.
    pub fn generated_virtual(mut self, expr: impl Into<Arc<str>>) -> Self {
        self.generated = Some((ComputedKind::OnDemand, expr.into()));
        self
    }

    /// Mark as auto-incrementing (replaces the old Serial/BigSerial types).
    ///
    /// Typically used with `DataType::Int32` or `DataType::Int64`.
    pub fn auto_assign(mut self) -> Self {
        self.auto_assign = true;
        self
    }
}
