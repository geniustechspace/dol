//! Lookup structural operations (`Operation::Lookup`).
//!
//! `Lookup` is the DOL term for the kind of index that exposes a fast
//! membership / equality query, but is implemented at the data layer (think
//! Mongo's `_id` index, a KV bucket's secondary lookup, or a relational
//! `UNIQUE INDEX` defined as a lookup table).

use smallvec::SmallVec;

use crate::operation::shared::StructuralVerb;
use crate::target::{Symbol, Target};

/// Lookup method.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LookupMethod {
    /// Hash-based lookup for O(1) equality checks.
    Hash,
    /// Tree-based lookup for range queries and ordering.
    Tree,
    /// Inverted index for full-text or multi-value lookups.
    Inverted,
    /// Backend-specific lookup method identified by interned name.
    Custom(Symbol),
}

/// Lookup structural operation.
///
/// Maps to Mongo's `_id` index, a KV bucket's secondary lookup, or a
/// relational `UNIQUE INDEX` defined as a lookup table.
///
/// # Examples
///
/// ```
/// use dol_ir::operation::{LookupMethod, LookupOp, StructuralVerb};
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::Operation;
///
/// // Create a hash lookup on the "users" collection
/// let op: Operation = LookupOp {
///     verb: StructuralVerb::Create,
///     target: Target::new(TargetKind::Document, Locator::new(Symbol::new(0))),
///     name: Symbol::new(1),
///     method: LookupMethod::Hash,
///     fields: smallvec::smallvec![Symbol::new(2)],
///     unique: true,
///     if_not_exists: false,
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::OpKind::Lookup);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LookupOp {
    /// `Create` / `Drop` / `Alter` / `Rename`.
    pub verb: StructuralVerb,
    /// Target the lookup is defined on.
    pub target: Target,
    /// Interned lookup name.
    pub name: Symbol,
    /// Lookup method (Hash, Tree, Inverted, …).
    pub method: LookupMethod,
    /// Fields included in the lookup key.
    pub fields: SmallVec<[Symbol; 2]>,
    /// Whether the lookup enforces uniqueness.
    pub unique: bool,
    /// Idempotency hint. For `Create`, suppress the failure when the lookup
    /// already exists. For `Drop`, suppress the failure when the lookup is
    /// missing. Backends without native support should pre-check.
    pub if_not_exists: bool,
}
