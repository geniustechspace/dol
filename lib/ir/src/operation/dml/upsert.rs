//! `Upsert` — insert-or-update with conflict resolution.

use dol_expr::ids::NodeId;

use crate::target::Target;

/// `Upsert` operation.
///
/// Body lives in an arena `ExprNode` of opcode [`ExprOp::Upsert`](dol_expr::expr::ExprOp::Upsert)
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
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(0))),
///     node: dol_expr::ids::NodeId::from_u32(1).unwrap(), // Real arena ids are non-zero
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::OpKind::Upsert);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Upsert {
    /// Target to upsert into.
    pub target: Target,
    /// Arena `NodeId` of the `ExprNode` of opcode [`ExprOp::Upsert`](dol_expr::expr::ExprOp::Upsert) body.
    pub node: NodeId,
}
