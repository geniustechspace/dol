//! `Replace` — full overwrite (HTTP PUT, S3 object overwrite, Mongo
//! `replaceOne`, file-tree write).
//!
//! Paired with [`Update`](crate::operation::update::Update).

use dol_expr::ids::NodeId;

use crate::target::Target;

/// Body of a `Replace`. The variants mirror [`InsertSource`](super::insert::InsertSource)
/// but `Replace` semantics are *full overwrite*.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ReplaceBody {
    /// Arena expression producing the replacement document/row.
    FromExpr(NodeId),
    /// Sub-query producing the replacement.
    FromQuery(NodeId),
    /// Runtime byte buffer.
    Bindings,
    /// Server-side path (file-tree / blob copy).
    FromPath(crate::target::Symbol),
}

/// `Replace` operation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Replace {
    pub target: Target,
    pub body: ReplaceBody,
    /// Arena `NodeId` for an optional `WHERE` predicate.
    pub filter: Option<NodeId>,
}
