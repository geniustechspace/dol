//! `Quota` — per-target / per-role rate-limiting and storage caps.

use crate::operation::shared::StructuralVerb;
use crate::target::{Symbol, Target};

/// What facet of usage is constrained.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum QuotaKind {
    /// Maximum bytes stored.
    Storage,
    /// Maximum row / document / object count.
    Count,
    /// Maximum operations per minute.
    Rate,
    /// Maximum concurrent connections / streams.
    Concurrency,
}

/// `Quota` operation.
///
/// Defines a usage limit (storage, count, rate, concurrency) on a target.
///
/// # Examples
///
/// ```
/// use dol_command::operation::{QuotaKind, QuotaOp, StructuralVerb};
/// use dol_command::target::{Locator, Symbol, Target, TargetKind};
/// use dol_command::operation::Operation;
///
/// // Limit the "uploads" blob bucket to 10GB
/// let op: Operation = QuotaOp {
///     verb: StructuralVerb::Create,
///     target: Target::new(TargetKind::Blob, Locator::new(Symbol::from_hash(0))),
///     name: Symbol::from_hash(1),
///     kind: QuotaKind::Storage,
///     limit: 10 * 1024 * 1024 * 1024, // 10GB in bytes
///     role: None,
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_command::operation::OpKind::Quota);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct QuotaOp {
    /// `Create` / `Drop` / `Alter`.
    pub verb: StructuralVerb,
    /// Target the quota applies to.
    pub target: Target,
    /// Interned quota rule name.
    pub name: Symbol,
    /// What facet of usage is constrained.
    pub kind: QuotaKind,
    /// Numeric limit value (interpretation depends on `kind`).
    pub limit: u64,
    /// Optional role / principal the quota is scoped to.
    pub role: Option<Symbol>,
}
