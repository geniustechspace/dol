//! `Insert` — append new tuples / documents / objects / records.
//!
//! Generic over [`TargetKind`](crate::target::TargetKind): inserting into a
//! `Relation` is a SQL `INSERT`, into a `Document` is a `insertOne` /
//! `insertMany`, into a `Blob` is `PutObject`, into a `FileTree` is a
//! file-create, and so on.

use dol_expr::ids::NodeId;
use smallvec::SmallVec;

use crate::target::Target;

/// Where the inserted payload comes from.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InsertSource {
    /// One or more rows of arena expressions (for relational/document inserts).
    Rows(SmallVec<[SmallVec<[NodeId; 4]>; 1]>),
    /// Insert from a sub-query (arena `Query` node).
    FromQuery(NodeId),
    /// Insert from runtime parameter bindings (the engine supplies the
    /// values at execution time). Used for object-store PUT and file-tree
    /// create when the body is a runtime byte buffer.
    Bindings,
    /// Insert from a server-side path. Replaces v1 `ObjectSource::FromPath`.
    FromPath(crate::target::Symbol),
    /// Insert from an arena expression evaluated at execution time. Replaces
    /// v1 `ObjectSource::FromExpr`.
    FromExpr(NodeId),
}

/// `Insert` operation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Insert {
    pub target: Target,
    pub source: InsertSource,
    /// Optional arena `NodeId` representing a `RETURNING` projection.
    pub returning: Option<NodeId>,
}
