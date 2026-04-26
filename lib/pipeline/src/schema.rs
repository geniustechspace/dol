//! Row schemas flowing through a pipeline.
//!
//! Schemas are intentionally lightweight: a `RowSchema` is an ordered list of
//! `(name, DataType, nullable)` tuples. Nominal entities live in `dol-schema`
//! and are referenced by name; this layer only tracks the structural shape
//! that flows along edges.

use dol_core::DataType;
use smallvec::SmallVec;

/// A single column in a row.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColumnSchema {
    /// Column name as it appears downstream.
    pub name: alloc::string::String,
    /// Column type.
    pub ty: DataType,
    /// Whether the column may carry NULLs.
    pub nullable: bool,
}

/// Ordered structural schema of a row.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RowSchema {
    /// Columns in left-to-right order.
    pub columns: SmallVec<[ColumnSchema; 8]>,
}

impl RowSchema {
    /// Empty schema (no columns).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Append a column; returns `self` for chaining.
    pub fn with_column(
        mut self,
        name: impl Into<alloc::string::String>,
        ty: DataType,
        nullable: bool,
    ) -> Self {
        self.columns.push(ColumnSchema {
            name: name.into(),
            ty,
            nullable,
        });
        self
    }

    /// Number of columns.
    pub fn len(&self) -> usize {
        self.columns.len()
    }

    /// `true` if there are no columns.
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }
}

extern crate alloc;
