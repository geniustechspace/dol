//! Identifiers for interned strings.
//!
//! String ids are content-addressed handles returned by
//! [`Interner`](super::Interner). They identify UTF-8 strings stored in the
//! string interner, not arbitrary byte slices and not process-local arena
//! positions.

use crate::ids::Id;

/// Tag for interned string ids.
///
/// This type is never instantiated. It exists only to make [`StrId`]
/// structurally distinct from other `Id<Tag>` aliases.
pub enum StrTag {}

/// Content-addressed id for an interned UTF-8 string.
///
/// `StrId` is derived from the leading 32 bits of the string's canonical hash,
/// folded away from zero so it can use the [`Id`] niche representation.
///
/// The same string should produce the same `StrId` across processes and
/// machines, assuming the same DOL hashing rules.
pub type StrId = Id<StrTag>;
