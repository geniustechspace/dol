//! Constraint types for model definitions.

use std::fmt;

/// Action to take when a referenced record is deleted or updated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FkAction {
    NoAction,
    Cascade,
    SetNull,
    Restrict,
    SetDefault,
}

impl fmt::Display for FkAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAction => write!(f, "NO ACTION"),
            Self::Cascade => write!(f, "CASCADE"),
            Self::SetNull => write!(f, "SET NULL"),
            Self::Restrict => write!(f, "RESTRICT"),
            Self::SetDefault => write!(f, "SET DEFAULT"),
        }
    }
}

/// How a generated (computed) column is defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GeneratedKind {
    /// `GENERATED ALWAYS AS (expr) STORED` — materialized on write.
    Stored,
    /// `GENERATED ALWAYS AS (expr) VIRTUAL` — computed on read.
    Virtual,
}

/// An inline foreign key reference on a single field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ForeignKeyRef {
    pub table: &'static str,
    pub column: &'static str,
    pub on_delete: FkAction,
    pub on_update: FkAction,
}

impl ForeignKeyRef {
    pub const fn new(table: &'static str, column: &'static str) -> Self {
        Self {
            table,
            column,
            on_delete: FkAction::NoAction,
            on_update: FkAction::NoAction,
        }
    }

    pub const fn on_delete(mut self, action: FkAction) -> Self {
        self.on_delete = action;
        self
    }

    pub const fn on_update(mut self, action: FkAction) -> Self {
        self.on_update = action;
        self
    }
}

/// A model-level constraint (composite UNIQUE, multi-field FK, CHECK, composite PK).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ModelConstraint {
    /// `UNIQUE (field1, field2, ...)`
    Unique(&'static [&'static str]),
    /// `FOREIGN KEY (fields) REFERENCES ref_model (ref_fields) ON DELETE action`
    ForeignKey {
        columns: &'static [&'static str],
        ref_table: &'static str,
        ref_columns: &'static [&'static str],
        on_delete: FkAction,
    },
    /// `CHECK (expression)`
    Check(&'static str),
    /// `PRIMARY KEY (field1, field2, ...)` — composite primary key.
    PrimaryKey(&'static [&'static str]),
}
