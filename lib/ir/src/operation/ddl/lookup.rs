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
    Hash,
    Tree,
    Inverted,
    Custom(Symbol),
}

/// Lookup structural operation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LookupOp {
    pub verb: StructuralVerb,
    pub target: Target,
    pub name: Symbol,
    pub method: LookupMethod,
    pub fields: SmallVec<[Symbol; 2]>,
    pub unique: bool,
}
