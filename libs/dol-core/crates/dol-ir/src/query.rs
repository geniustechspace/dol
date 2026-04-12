//! Query IR — the canonical representation of a data retrieval operation.

use super::ModelRef;
use dol_expr::{Expr, OrderByExpr};

/// How to retrieve data from a source.
#[derive(Debug, Clone)]
pub struct QueryIR {
    pub source: ModelRef,
    pub projections: Vec<Expr>,
    pub joins: Vec<JoinIR>,
    pub filters: Vec<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Vec<Expr>,
    pub order_by: Vec<OrderByExpr>,
    pub offset: Option<OffsetLimit>,
    pub limit: Option<OffsetLimit>,
    pub distinct: bool,
    pub distinct_on: Vec<String>,
    pub lock_mode: Option<LockMode>,
}

/// Offset/Limit can be either a bind parameter or a literal value.
#[derive(Debug, Clone)]
pub enum OffsetLimit {
    /// A bind parameter placeholder ($N).
    Param,
    /// A literal value.
    Value(u64),
}

/// A JOIN clause in a query.
#[derive(Debug, Clone)]
pub struct JoinIR {
    pub join_type: JoinType,
    pub target: ModelRef,
    pub on_conditions: Vec<(String, String)>,
}

/// The type of join.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// Lock mode for SELECT ... FOR UPDATE / FOR SHARE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockMode {
    ForUpdate,
    ForShare,
    ForUpdateNoWait,
    ForUpdateSkipLocked,
    ForShareNoWait,
    ForShareSkipLocked,
}

/// Set operation kind for compound queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOpKind {
    Union,
    UnionAll,
    Intersect,
    IntersectAll,
    Except,
    ExceptAll,
}

/// A compound query (set operations: UNION, INTERSECT, EXCEPT).
#[derive(Debug, Clone)]
pub struct CompoundQueryIR {
    pub base: Box<QueryIR>,
    pub operations: Vec<(SetOpKind, QueryIR)>,
    pub order_by: Vec<OrderByExpr>,
    pub offset: Option<OffsetLimit>,
    pub limit: Option<OffsetLimit>,
}
