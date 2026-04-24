//! Owned, serde-friendly schema constraint types.
//!
//! These types are the canonical home for `Entity` / `Field` constraints in
//! the data-model layer. They use owned strings (`Arc<str>`) so they can
//! implement `Deserialize` and survive round-trips through serde.
//!
//! The `Copy` unit enums (`FkAction`, `GeneratedKind`) are re-exported from
//! `dol-ir` to avoid duplication.

use std::sync::Arc;

pub use dol_ir::constraint::{FkAction, GeneratedKind};

/// An inline foreign key reference on a single field.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ForeignKeyRef {
    pub table: Arc<str>,
    pub column: Arc<str>,
    pub on_delete: FkAction,
    pub on_update: FkAction,
}

impl ForeignKeyRef {
    pub fn new(table: impl Into<Arc<str>>, column: impl Into<Arc<str>>) -> Self {
        Self {
            table: table.into(),
            column: column.into(),
            on_delete: FkAction::NoAction,
            on_update: FkAction::NoAction,
        }
    }

    pub fn on_delete(mut self, action: FkAction) -> Self {
        self.on_delete = action;
        self
    }

    pub fn on_update(mut self, action: FkAction) -> Self {
        self.on_update = action;
        self
    }
}

/// A model-level constraint (composite UNIQUE, multi-field FK, CHECK, composite PK).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EntityConstraint {
    /// `UNIQUE (field1, field2, ...)`
    Unique(Vec<Arc<str>>),
    /// `FOREIGN KEY (fields) REFERENCES ref_model (ref_fields) ON DELETE action`
    ForeignKey {
        columns: Vec<Arc<str>>,
        ref_table: Arc<str>,
        ref_columns: Vec<Arc<str>>,
        on_delete: FkAction,
    },
    /// `CHECK (expression)`
    Check(Arc<str>),
    /// `PRIMARY KEY (field1, field2, ...)` — composite primary key.
    PrimaryKey(Vec<Arc<str>>),
}

impl EntityConstraint {
    /// Build a `UNIQUE` constraint from any iterable of string-likes.
    pub fn unique<I, S>(cols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Arc<str>>,
    {
        Self::Unique(cols.into_iter().map(Into::into).collect())
    }

    /// Build a `PRIMARY KEY` constraint from any iterable of string-likes.
    pub fn primary_key<I, S>(cols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Arc<str>>,
    {
        Self::PrimaryKey(cols.into_iter().map(Into::into).collect())
    }

    /// Build a `CHECK` constraint.
    pub fn check(expr: impl Into<Arc<str>>) -> Self {
        Self::Check(expr.into())
    }

    /// Build a `FOREIGN KEY` constraint.
    pub fn foreign_key<I, J, S, T>(
        columns: I,
        ref_table: impl Into<Arc<str>>,
        ref_columns: J,
        on_delete: FkAction,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        J: IntoIterator<Item = T>,
        S: Into<Arc<str>>,
        T: Into<Arc<str>>,
    {
        Self::ForeignKey {
            columns: columns.into_iter().map(Into::into).collect(),
            ref_table: ref_table.into(),
            ref_columns: ref_columns.into_iter().map(Into::into).collect(),
            on_delete,
        }
    }
}
