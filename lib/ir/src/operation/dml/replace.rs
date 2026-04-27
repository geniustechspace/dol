//! `Replace` — full overwrite (HTTP PUT, S3 object overwrite, Mongo
//! `replaceOne`, file-tree write).
//!
//! Paired with [`Update`](crate::operation::Update).

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
///
/// Full overwrite of a target object (HTTP PUT, S3 object overwrite,
/// Mongo `replaceOne`, file-tree write).
///
/// # Examples
///
/// ```
/// use dol_ir::operation::{Replace, ReplaceBody};
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::Operation;
///
/// // Replace a blob at s3://artifacts/build.log
/// let op: Operation = Replace {
///     target: Target::new(TargetKind::Blob, Locator::new(Symbol::new(0))),
///     body: ReplaceBody::Bindings,
///     filter: None,
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::OpKind::Replace);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Replace {
    /// Target to replace.
    pub target: Target,
    /// Replacement data source.
    pub body: ReplaceBody,
    /// Arena `NodeId` for an optional `WHERE` predicate.
    pub filter: Option<NodeId>,
}
