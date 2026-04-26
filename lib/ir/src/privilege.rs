//! [`Privilege`] — values granted / revoked by `Operation::Grant` /
//! `Operation::Revoke`.
//!
//! This is the only piece of the v1 ACL surface kept after the v2 cleanup.
//! Its v1 wrappers (`Grant { privilege, on_target, to_role }`,
//! `Revoke`, `DefinePolicy`) are gone; v2 carries privileges in
//! [`GrantV2`](crate::operation::GrantV2) / [`RevokeV2`](crate::operation::RevokeV2)
//! against a [`Target`](crate::target::Target).

use alloc::string::String;

extern crate alloc;

/// Privilege types that can be granted or revoked.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Privilege {
    Select,
    Insert,
    Update,
    Delete,
    All,
    Usage,
    Create,
    Connect,
    Custom(String),
}
