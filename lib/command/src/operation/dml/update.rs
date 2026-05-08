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
/// arena `ExprNode` of opcode [`ExprOp::Update`](dol_expr::expr::ExprOp::Update) node
/// referenced by [`Update::node`]. Backends pull `node` and walk the arena.
///
/// # Examples
///
/// ```
/// use dol_command::operation::Update;
/// use dol_command::target::{Locator, Symbol, Target, TargetKind};
/// use dol_command::operation::Operation;
///
/// // UPDATE users SET name = 'Alice' WHERE id = 1
/// let op: Operation = Update {
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(0))),
///     node: dol_expr::ids::NodeId::from_u32(1).unwrap(), // Real arena ids are non-zero
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_command::operation::OpKind::Update);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Update {
    /// Target to update.
    pub target: Target,
    /// Arena `NodeId` of the `ExprNode` of opcode [`ExprOp::Update`](dol_expr::expr::ExprOp::Update) body.
    pub node: NodeId,
}
