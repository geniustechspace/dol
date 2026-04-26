//! `Update` — partial mutation (SQL `UPDATE ... SET`, Mongo `updateOne`,
//! KV partial overwrite, file-tree rename).
//!
//! `Update` is paired with [`Replace`](crate::operation::replace::Replace).
//! `Update` is *partial*; `Replace` is *full*. The v1 `Patch` semantics are
//! folded into `Update` (see RFC §1).

use dol_expr::ids::NodeId;
use smallvec::SmallVec;

use crate::target::{Symbol, Target};

/// One assignment in an `Update::sets` list.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UpdateAssignment {
    /// Target field / key / path.
    pub field: Symbol,
    /// Arena `NodeId` for the assigned expression.
    pub value: NodeId,
}

/// `Update` operation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Update {
    pub target: Target,
    pub sets: SmallVec<[UpdateAssignment; 4]>,
    /// Arena `NodeId` for the `WHERE` predicate.
    pub filter: Option<NodeId>,
    /// Arena `NodeId` for `RETURNING`.
    pub returning: Option<NodeId>,
}
