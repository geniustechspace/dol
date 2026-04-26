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
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Describe {
    pub target: Target,
    pub facet: DescribeFacet,
}
