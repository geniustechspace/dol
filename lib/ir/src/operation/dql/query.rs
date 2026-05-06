//! `Query` — read tuples / documents / objects / files.

use dol_expr::ids::NodeId;

use crate::target::Target;

/// `Query` operation.
///
/// The body of the query (projection, joins, filters, sort, limit) lives in
/// the expression arena under `node`. Backends pull `node` and walk the
/// arena.
///
/// # Examples
///
/// ```
/// use dol_ir::operation::Query;
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::Operation;
///
/// // SELECT * FROM users
/// let op: Operation = Query {
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(0))),
///     node: None, // Would be set to arena NodeId for real query
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::OpKind::Query);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Query {
    /// Target to query.
    pub target: Target,
    /// Arena `NodeId` for the query body (a `QueryNode` / projection tree).
    pub node: Option<NodeId>,
}
