//! `Delete` — remove rows / documents / objects / files.

use dol_expr::ids::NodeId;

use crate::target::Target;

/// `Delete` operation.
///
/// The WHERE filter + RETURNING projection live in an arena
/// [`ExprNode::Delete`](dol_expr::expr::ExprNode::Delete) referenced by
/// [`Delete::node`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Delete {
    pub target: Target,
    /// Arena `NodeId` of the [`ExprNode::Delete`] body.
    pub node: NodeId,
}
