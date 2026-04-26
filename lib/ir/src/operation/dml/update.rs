//! `Update` — partial mutation (SQL `UPDATE ... SET`, Mongo `updateOne`,
//! KV partial overwrite, file-tree rename).
//!
//! Paired with [`Replace`](crate::operation::Replace). `Update` is
//! *partial*; `Replace` is *full*.

use dol_expr::ids::NodeId;

use crate::target::Target;

/// `Update` operation.
///
/// The SET assignments + WHERE filter + RETURNING projection live in an
/// arena [`ExprNode::Update`](dol_expr::expr::ExprNode::Update) node
/// referenced by [`Update::node`]. Backends pull `node` and walk the arena.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Update {
    pub target: Target,
    /// Arena `NodeId` of the [`ExprNode::Update`] body.
    pub node: NodeId,
}
