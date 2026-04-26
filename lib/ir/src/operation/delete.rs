//! `Delete` — remove rows / documents / objects / files.

use dol_expr::ids::NodeId;

use crate::target::Target;

/// `Delete` operation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Delete {
    pub target: Target,
    /// Arena `NodeId` for the `WHERE` predicate.
    pub filter: Option<NodeId>,
    /// Arena `NodeId` for `RETURNING`.
    pub returning: Option<NodeId>,
}
