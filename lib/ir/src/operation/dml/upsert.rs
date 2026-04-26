//! `Upsert` — insert-or-update with conflict resolution.

use dol_expr::ids::NodeId;

use crate::target::Target;

/// `Upsert` operation.
///
/// Body lives in an arena [`ExprNode::Upsert`](dol_expr::expr::ExprNode::Upsert)
/// node; the arena `UpsertNode` carries the columns, values, conflict
/// clause, and returning list.
///
/// # Examples
///
/// ```
/// use dol_ir::operation::Upsert;
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::Operation;
///
/// // INSERT INTO users (...) ON CONFLICT DO UPDATE SET ...
/// let op: Operation = Upsert {
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::new(0))),
///     node: 0, // Would be a real arena NodeId
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::OpKind::Upsert);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Upsert {
    /// Target to upsert into.
    pub target: Target,
    /// Arena `NodeId` of the [`ExprNode::Upsert`] body.
    pub node: NodeId,
}
