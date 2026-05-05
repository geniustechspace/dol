//! `Delete` — remove rows / documents / objects / files.

use dol_expr::ids::NodeId;

use crate::target::Target;

/// `Delete` operation.
///
/// The WHERE filter + RETURNING projection live in an arena
/// [`ExprNode::Delete`](dol_expr::expr::ExprNode::Delete) referenced by
/// [`Delete::node`].
///
/// # Examples
///
/// ```
/// use dol_ir::operation::Delete;
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::Operation;
///
/// // DELETE FROM users WHERE id = 1
/// let op: Operation = Delete {
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(0))),
///     node: dol_expr::ids::NodeId::from_u32(1).unwrap(), // Real arena ids are non-zero
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::OpKind::Delete);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Delete {
    /// Target to delete from.
    pub target: Target,
    /// Arena `NodeId` of the [`ExprNode::Delete`](dol_expr::expr::ExprNode::Delete) body.
    pub node: NodeId,
}
