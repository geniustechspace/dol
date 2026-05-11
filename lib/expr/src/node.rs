#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum Order {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum LockHint {
    ForUpdate,
    ForShare,
    SkipLocked,
    NoWait,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum ConflictClause {
    DoNothing,
    DoUpdate {
        assignments: SmallVec<[(StrId, NodeId); 4]>,
    },
}

// ═══════════════════════════════════════════════════════════════════════════
// Pooled side-payload structs (referenced from `ExprNode` via typed ids)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct JoinNode {
    pub source: StrId,
    pub alias: Option<StrId>,
    pub join_type: JoinType,
    /// The `ON` condition. `None` for `CROSS JOIN` and other joins
    /// without a predicate (replaces the previous
    /// `NULL_NODE = u32::MAX` sentinel).
    pub on: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct QueryNode {
    pub from: StrId,
    pub alias: Option<StrId>,
    pub joins: SmallVec<[JoinNode; 2]>,
    pub filter: Option<NodeId>,
    pub columns: SmallVec<[NodeId; 8]>,
    pub group_by: SmallVec<[NodeId; 4]>,
    pub having: Option<NodeId>,
    pub order_by: SmallVec<[(NodeId, Order); 4]>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub lock: Option<LockHint>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct InsertNode {
    pub target: StrId,
    pub columns: SmallVec<[StrId; 8]>,
    pub values: SmallVec<[NodeId; 8]>,
    pub returning: SmallVec<[NodeId; 4]>,
    pub conflict: Option<ConflictClause>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct UpdateNode {
    pub fields: SmallVec<[StrId; 8]>,
    pub values: SmallVec<[NodeId; 8]>,
    pub filter: Option<NodeId>,
    pub returning: SmallVec<[NodeId; 4]>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct DeleteNode {
    pub filter: Option<NodeId>,
    pub returning: SmallVec<[NodeId; 4]>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct UpsertNode {
    pub fields: SmallVec<[StrId; 8]>,
    pub values: SmallVec<[NodeId; 8]>,
    pub returning: SmallVec<[NodeId; 4]>,
    pub conflict: Option<ConflictClause>,
}
