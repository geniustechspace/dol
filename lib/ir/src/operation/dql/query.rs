//! `Query` — read tuples / documents / objects / files.

use dol_expr::ids::NodeId;

use crate::target::Target;

/// `Query` operation.
///
/// The body of the query (projection, joins, filters, sort, limit) lives in
/// the expression arena under `node`. Backends pull `node` and walk the
/// arena.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Query {
    pub target: Target,
    /// Arena `NodeId` for the query body (a `QueryNode` / projection tree).
    pub node: Option<NodeId>,
}
