//! [`Privilege`] — values granted / revoked by `Operation::Grant` /
//! `Operation::Revoke`.
//!
//! Privileges are carried in [`Grant`](crate::operation::Grant) /
//! [`Revoke`](crate::operation::Revoke) against a
//! [`Target`](crate::target::Target).

extern crate alloc;

use crate::target::Symbol;

/// Privilege types that can be granted or revoked.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Privilege {
    /// Read rows / objects from the target.
    Select,
    /// Insert new rows / objects.
    Insert,
    /// Modify existing rows / objects in place.
    Update,
    /// Remove rows / objects.
    Delete,
    /// All privileges supported by the backend for this target kind.
    All,
    /// Use the target without read/write (e.g. `USAGE` on a SQL schema).
    Usage,
    /// Create new objects within a container target.
    Create,
    /// Open a connection / session against the target.
    Connect,
    /// Backend-specific privilege identified by an interned name. Resolve
    /// against the surrounding program's [`Interner`](dol_expr::Interner)
    /// to recover its textual form.
    Custom(Symbol),
}
