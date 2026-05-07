//! `Append` — append-only writes (stream topics, append logs, immutable
//! file-tree writes).
//!
//! Distinct from `Insert` because backends model append-only semantics
//! differently (no auto-id, no upsert, no rollback within a partition).

use dol_expr::ids::NodeId;

use crate::operation::dml::insert::InsertSource;
use crate::target::Target;

/// `Append` operation.
///
/// Append-only write to a stream topic, append log, or immutable file.
///
/// # Examples
///
/// ```
/// use dol_ir::operation::{Append, InsertSource};
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::operation::Operation;
///
/// // Append to kafka://events.users topic
/// let op: Operation = Append {
///     target: Target::new(TargetKind::StreamTopic, Locator::new(Symbol::from_hash(0))),
///     source: InsertSource::Bindings,
///     partition_key: None,
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::operation::OpKind::Append);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Append {
    /// Target to append to (typically a stream topic).
    pub target: Target,
    /// Where the appended payload comes from.
    pub source: InsertSource,
    /// Optional partition / shard key expression.
    pub partition_key: Option<NodeId>,
}
