//! Verbs shared by every structural / governance operation.
//!
//! Noun-shaped operation payloads ([`SchemaOp`](super::super::ddl::SchemaOp),
//! [`FieldOp`](super::super::ddl::FieldOp),
//! [`IndexOp`](super::super::ddl::IndexOp),
//! [`LookupOp`](super::super::ddl::LookupOp),
//! [`PolicyOp`](super::super::acl::PolicyOp),
//! [`MaskOp`](super::super::acl::MaskOp),
//! [`QuotaOp`](super::super::acl::QuotaOp),
//! [`AuditOp`](super::super::acl::AuditOp))
//! all carry a [`StructuralVerb`] to say *what* is being done to the
//! underlying object.

/// The verb applied by a structural / governance operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StructuralVerb {
    /// Create the object if it does not exist.
    Create,
    /// Drop the object.
    Drop,
    /// Alter the object's body.
    Alter,
    /// Rename the object.
    Rename,
    /// Empty the object's contents (data-bearing targets only).
    Truncate,
}
