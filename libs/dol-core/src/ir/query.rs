//! Query IR — the canonical representation of a data retrieval operation.

use super::EntityRef;
use crate::expr::{Expr, OrderByExpr};

/// How to retrieve data from a source.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QueryIR<'a> {
    pub source: EntityRef,
    pub projections: Vec<Expr<'a>>,
    pub joins: Vec<JoinIR>,
    pub filters: Vec<Expr<'a>>,
    pub group_by: Vec<Expr<'a>>,
    pub having: Vec<Expr<'a>>,
    pub order_by: Vec<OrderByExpr<'a>>,
    pub offset: Option<OffsetLimit>,
    pub limit: Option<OffsetLimit>,
    pub distinct: bool,
    pub distinct_on: Vec<String>,
    pub lock_mode: Option<LockMode>,
}

/// Offset/Limit can be either a bind parameter or a literal value.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OffsetLimit {
    /// A bind parameter placeholder ($N).
    Param,
    /// A literal value.
    Value(u64),
}

/// A JOIN clause in a query.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct JoinIR {
    pub join_type: JoinType,
    pub target: EntityRef,
    pub on_conditions: Vec<(String, String)>,
}

/// The type of join.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// Lock mode for SELECT ... FOR UPDATE / FOR SHARE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SetOpKind {
    Union,
    UnionAll,
    Intersect,
    IntersectAll,
    Except,
    ExceptAll,
}

/// A compound query (set operations: UNION, INTERSECT, EXCEPT).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CompoundQueryIR<'a> {
    pub base: Box<QueryIR<'a>>,
    pub operations: Vec<(SetOpKind, QueryIR<'a>)>,
    pub order_by: Vec<OrderByExpr<'a>>,
    pub offset: Option<OffsetLimit>,
    pub limit: Option<OffsetLimit>,
}
