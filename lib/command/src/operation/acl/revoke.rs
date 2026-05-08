//! `Revoke` — remove privileges on a target from a role.

use smallvec::SmallVec;

use crate::privilege::Privilege;
use crate::target::{Symbol, Target};

/// `Revoke` operation: withdraw one or more privileges on a target from a set
/// of roles.
///
/// # What this represents
///
/// Maps to SQL `REVOKE <privileges> ON <target> FROM <roles> [CASCADE]`, to
/// a document-store ACL "deny" / "remove" rule, or to removing a bucket
/// policy statement, depending on the [`crate::target::TargetKind`] of
/// [`Self::target`].
///
/// # Examples
///
/// ```
/// use dol_command::operation::Revoke;
/// use dol_command::target::{Locator, Symbol, Target, TargetKind};
/// use dol_command::operation::Operation; use dol_command::privilege::Privilege;
/// use smallvec::smallvec;
///
/// // REVOKE INSERT ON users FROM app_role
/// let op: Operation = Revoke {
///     privileges: smallvec![Privilege::Insert],
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(0))),
///     roles: smallvec![Symbol::from_hash(1)],
///     cascade: false,
/// }
/// .into();
/// assert_eq!(op.kind(), dol_command::operation::OpKind::Revoke);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Revoke {
    /// Privileges being revoked.
    pub privileges: SmallVec<[Privilege; 2]>,
    /// Target the privileges apply to.
    pub target: Target,
    /// Roles or principals losing the privileges.
    pub roles: SmallVec<[Symbol; 1]>,
    /// When `true`, also revoke from any role that received the privilege
    /// transitively via `WITH GRANT OPTION`.
    pub cascade: bool,
}
