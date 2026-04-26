//! `Quota` — per-target / per-role rate-limiting and storage caps.

use crate::operation::shared::StructuralVerb;
use crate::target::{Symbol, Target};

/// What facet of usage is constrained.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QuotaOp {
    pub verb: StructuralVerb,
    pub target: Target,
    pub name: Symbol,
    pub kind: QuotaKind,
    pub limit: u64,
    /// Optional role / principal the quota is scoped to.
    pub role: Option<Symbol>,
}
