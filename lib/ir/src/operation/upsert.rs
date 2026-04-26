//! `Upsert` — insert-or-update with conflict resolution.

use dol_expr::ids::NodeId;

use crate::target::Target;

/// `Upsert` operation.
///
/// Body lives in an arena [`ExprNode::Upsert`](dol_expr::expr::ExprNode::Upsert)
/// node; the arena `UpsertNode` carries the columns, values, conflict
/// clause, and returning list.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Upsert {
    pub target: Target,
    /// Arena `NodeId` of the [`ExprNode::Upsert`] body.
    pub node: NodeId,
}
