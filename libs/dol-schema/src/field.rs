//! Field definition within an Entity.
//!
//! In DOL, a **Field** is a named property with a type, optional constraints,
//! and an optional default expression. The DOL philosophy:
//! - Fields are **required** (NOT NULL) by default
//! - Use `.optional()` or `.nullable()` to make them nullable
//! - Use `.required()` as a no-op self-documenting marker

use super::constraint::{FkAction, ForeignKeyRef, GeneratedKind};
pub use dol_types::DataType;

/// A field definition within an entity.
///
/// Fields support the full range of constraints and references:
/// - `primary_key()` — marks as part of the primary key
/// - `nullable()` / `optional()` — allows NULL values (NOT NULL by default)
/// - `unique()` — adds a UNIQUE constraint
/// - `default(expr)` — sets a DEFAULT expression rendered in DDL
/// - `references(entity, field, on_delete, on_update)` — inline foreign key
/// - `check(expr)` — inline CHECK constraint
/// - `index()` — hints that this field should be indexed
/// - `collation(name)` — overrides the collation for this field
/// - `generated_stored(expr)` / `generated_virtual(expr)` — computed fields
/// - `auto_increment()` — marks as auto-incrementing (replaces Serial/BigSerial)
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Field {
    pub name: &'static str,
    pub data_type: DataType,
    pub primary_key: bool,
    pub nullable: bool,
    pub has_default: bool,
    pub default_expr: Option<&'static str>,
    pub unique: bool,
    pub references: Option<ForeignKeyRef>,
    /// Inline CHECK constraint expression.
    pub check: Option<&'static str>,
    /// Human-readable description / comment.
    pub comment: Option<&'static str>,
    /// Collation override (e.g. `"C"`, `"en_US.UTF-8"`).
    pub collation: Option<&'static str>,
    /// Generated (computed) field: `(kind, expression)`.
    pub generated: Option<(GeneratedKind, &'static str)>,
    /// Hint that this field should be indexed (for schema generation tooling).
    pub indexed: bool,
    /// Auto-incrementing field (replaces the old Serial/BigSerial types).
    pub auto_increment: bool,
}

impl Field {
    pub fn new(name: &'static str, data_type: DataType) -> Self {
        Self {
            name,
            data_type,
            primary_key: false,
            nullable: false,
            has_default: false,
            default_expr: None,
            unique: false,
            references: None,
            check: None,
            comment: None,
            collation: None,
            generated: None,
            indexed: false,
            auto_increment: false,
        }
    }

    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
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
    pub fn default(mut self, expr: &'static str) -> Self {
        self.has_default = true;
        self.default_expr = Some(expr);
        self
    }

    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    /// Add an inline REFERENCES constraint with ON DELETE and ON UPDATE actions.
    pub fn references(
        mut self,
        table: &'static str,
        column: &'static str,
        on_delete: FkAction,
        on_update: FkAction,
    ) -> Self {
        self.references = Some(ForeignKeyRef {
            table,
            column,
            on_delete,
            on_update,
        });
        self
    }

    /// Add a fully-configured foreign key reference.
    pub fn references_full(mut self, fk: ForeignKeyRef) -> Self {
        self.references = Some(fk);
        self
    }

    /// Add an inline CHECK constraint expression.
    pub fn check(mut self, expr: &'static str) -> Self {
        self.check = Some(expr);
        self
    }

    /// Set a human-readable comment / description for this field.
    pub fn comment(mut self, text: &'static str) -> Self {
        self.comment = Some(text);
        self
    }

    /// Override the collation for this field.
    pub fn collation(mut self, collation: &'static str) -> Self {
        self.collation = Some(collation);
        self
    }

    /// Hint that this field should be indexed.
    pub fn index(mut self) -> Self {
        self.indexed = true;
        self
    }

    /// Mark as a stored generated (computed) field.
    pub fn generated_stored(mut self, expr: &'static str) -> Self {
        self.generated = Some((GeneratedKind::Stored, expr));
        self
    }

    /// Mark as a virtual generated (computed) field.
    pub fn generated_virtual(mut self, expr: &'static str) -> Self {
        self.generated = Some((GeneratedKind::Virtual, expr));
        self
    }

    /// Mark as auto-incrementing (replaces the old Serial/BigSerial types).
    ///
    /// Typically used with `DataType::Int32` or `DataType::Int64`.
    pub fn auto_increment(mut self) -> Self {
        self.auto_increment = true;
        self
    }
}
