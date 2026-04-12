//! Mutation IR — canonical representation of data modification operations.

use super::ModelRef;
use dol_expr::Expr;

/// Insert new records into a model.
#[derive(Debug, Clone)]
pub struct InsertIR {
    pub target: ModelRef,
    pub fields: Vec<String>,
    pub row_count: usize,
    pub returning: Vec<String>,
}

/// Insert from a subquery: `INSERT INTO ... SELECT ...`
#[derive(Debug, Clone)]
pub struct InsertSelectIR {
    pub target: ModelRef,
    pub fields: Vec<String>,
    pub source_query: String,
    pub returning: Vec<String>,
}

/// Update existing records in a model.
#[derive(Debug, Clone)]
pub struct UpdateIR {
    pub target: ModelRef,
    pub assignments: Vec<(String, Expr)>,
    pub filters: Vec<Expr>,
    pub returning: Vec<String>,
}

/// Remove records from a model.
#[derive(Debug, Clone)]
pub struct RemoveIR {
    pub target: ModelRef,
    pub filters: Vec<Expr>,
    pub returning: Vec<String>,
}

/// Upsert (insert or update on conflict).
#[derive(Debug, Clone)]
pub struct UpsertIR {
    pub target: ModelRef,
    pub fields: Vec<String>,
    pub conflict_fields: Vec<String>,
    pub conflict_constraint: Option<String>,
    pub update_fields: Vec<String>,
    pub do_nothing: bool,
    pub conflict_filters: Vec<Expr>,
    pub returning: Vec<String>,
}
