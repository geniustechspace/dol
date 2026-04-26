//! `Insert` — append new tuples / documents / objects / records.
//!
//! Generic over [`TargetKind`](crate::target::TargetKind): inserting into a
//! `Relation` is a SQL `INSERT`, into a `Document` is `insertOne`, into a
//! `Blob` is `PutObject`, into a `FileTree` is a file-create, etc.

use dol_expr::ids::NodeId;

use crate::target::{Symbol, Target};

/// Where the inserted payload comes from.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InsertSource {
    /// Tabular insert body — `NodeId` points to an arena
    /// [`ExprNode::Insert`](dol_expr::expr::ExprNode::Insert) carrying
    /// columns + values + conflict clause. Used for `Relation`, `Document`,
    /// `KeyValue` targets.
    Node(NodeId),
    /// Insert from a sub-query (arena `NodeId` pointing at
    /// [`ExprNode::Query`](dol_expr::expr::ExprNode::Query)).
    FromQuery(NodeId),
    /// Insert from runtime parameter bindings (engine supplies the bytes /
    /// document body at execution time). Used for `Blob` / `FileTree` /
    /// `ApiResource` targets.
    Bindings,
    /// Insert from a server-side path (file copy, blob upload-from-path).
    FromPath(Symbol),
    /// Insert from an arena expression evaluated at execution time.
    FromExpr(NodeId),
}

/// `Insert` operation.
///
/// Append new tuples / documents / objects / records to a target.
///
/// # Examples
///
/// ```
/// use dol_ir::operation::{Insert, InsertSource};
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::Operation;
///
/// // INSERT INTO users (...) VALUES (...)
/// let op: Operation = Insert {
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::new(0))),
///     source: InsertSource::Bindings,
///     returning: None,
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::OpKind::Insert);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Insert {
    /// Target to insert into.
    pub target: Target,
    /// Where the inserted payload comes from.
    pub source: InsertSource,
    /// Optional arena `NodeId` representing a `RETURNING` projection. For
    /// `Source::Node`, the returning list inside the arena `InsertNode`
    /// is canonical and this field is `None`.
    pub returning: Option<NodeId>,
}
