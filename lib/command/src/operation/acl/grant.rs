//! `Grant` — bestow privileges on a target to a role.
//!
//! Generalises [`crate::privilege::Privilege`] over any [`crate::target::TargetKind`].
//! The [`crate::privilege::Privilege`] enum is reused verbatim from the privilege module.

use smallvec::SmallVec;

use crate::privilege::Privilege;
use crate::target::{Symbol, Target};

/// `Grant` operation: bestow one or more privileges on a target to a set of
/// roles.
///
/// # What this represents
///
/// Maps to SQL `GRANT <privileges> ON <target> TO <roles> [WITH GRANT OPTION]`,
/// to a document-store ACL "allow" rule, or to an object-store bucket policy
/// statement, depending on the [`crate::target::TargetKind`] of [`Self::target`].
///
/// # Examples
///
/// ```
/// use dol_command::operation::Grant;
/// use dol_command::target::{Locator, Symbol, Target, TargetKind};
/// use dol_command::operation::Operation; use dol_command::privilege::Privilege;
/// use smallvec::smallvec;
///
/// // GRANT SELECT, INSERT ON users TO app_role
/// let op: Operation = Grant {
///     privileges: smallvec![Privilege::Select, Privilege::Insert],
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(0))),
///     roles: smallvec![Symbol::from_hash(1)],
///     with_grant_option: false,
/// }
/// .into();
/// assert_eq!(op.kind(), dol_command::operation::OpKind::Grant);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Grant {
    /// Privileges being granted (e.g. `Select`, `Insert`).
    pub privileges: SmallVec<[Privilege; 2]>,
    /// Target the privileges apply to (relation, blob bucket, stream topic, …).
    pub target: Target,
    /// Roles or principals receiving the privileges.
    pub roles: SmallVec<[Symbol; 1]>,
    /// When `true`, granted roles may re-grant the same privileges to others.
    pub with_grant_option: bool,
}
