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
///
/// # Examples
///
/// ```
/// use dol_ir::operation::Update;
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::Operation;
///
/// // UPDATE users SET name = 'Alice' WHERE id = 1
/// let op: Operation = Update {
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::new(0))),
///     node: 0, // Would be a real arena NodeId
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::OpKind::Update);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Update {
    /// Target to update.
    pub target: Target,
    /// Arena `NodeId` of the [`ExprNode::Update`](dol_expr::expr::ExprNode::Update) body.
    pub node: NodeId,
}
