//! `Describe` — schema introspection (`information_schema`, `SHOW TABLES`,
//! Mongo `$collStats`, S3 `ListBuckets`).

use crate::target::Target;

/// What facet of the target to describe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DescribeFacet {
    /// Top-level shape.
    Shape,
    /// Field / column metadata.
    Fields,
    /// Index / lookup metadata.
    Indexes,
    /// Policies / masks / quotas / audit definitions.
    Acl,
    /// Statistics (row counts, sizes).
    Stats,
}

/// `Describe` operation.
///
/// Returns schema metadata about a target (shape, fields, indexes, ACLs,
/// or statistics).
///
/// # Examples
///
/// ```
/// use dol_ir::operation::{Describe, DescribeFacet};
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::Operation;
///
/// // DESCRIBE users (field metadata)
/// let op: Operation = Describe {
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(0))),
///     facet: DescribeFacet::Fields,
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::OpKind::Describe);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Describe {
    /// Target to describe.
    pub target: Target,
    /// Which metadata facet to return.
    pub facet: DescribeFacet,
}
