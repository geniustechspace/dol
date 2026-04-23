//! Mutation operations — canonical representation of data modification operations.

use super::EntityRef;
use crate::expr::Expr;

/// Insert new records into a model.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Insert {
    pub target: EntityRef,
    pub fields: Vec<String>,
    pub row_count: usize,
    pub returning: Vec<String>,
}

/// Insert from a subquery: `INSERT INTO ... SELECT ...`
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InsertSelect {
    pub target: EntityRef,
    pub fields: Vec<String>,
    pub source_query: String,
    pub returning: Vec<String>,
}

/// Update existing records in a model.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Update<'a> {
    pub target: EntityRef,
    pub assignments: Vec<(String, Expr<'a>)>,
    pub filters: Vec<Expr<'a>>,
    pub returning: Vec<String>,
}

/// Remove records from a model.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Remove<'a> {
    pub target: EntityRef,
    pub filters: Vec<Expr<'a>>,
    pub returning: Vec<String>,
}

/// Upsert (insert or update on conflict).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Upsert<'a> {
    pub target: EntityRef,
    pub fields: Vec<String>,
    pub conflict_fields: Vec<String>,
    pub conflict_constraint: Option<String>,
    pub update_fields: Vec<String>,
    pub do_nothing: bool,
    pub conflict_filters: Vec<Expr<'a>>,
    pub returning: Vec<String>,
}
