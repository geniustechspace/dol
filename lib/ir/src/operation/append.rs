//! `Append` — append-only writes (stream topics, append logs, immutable
//! file-tree writes).
//!
//! Distinct from `Insert` because backends model append-only semantics
//! differently (no auto-id, no upsert, no rollback within a partition).

use dol_expr::ids::NodeId;

use crate::operation::insert::InsertSource;
use crate::target::Target;

/// `Append` operation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Append {
    pub target: Target,
    pub source: InsertSource,
    /// Optional partition / shard key expression.
    pub partition_key: Option<NodeId>,
}
