//! `Revoke` — remove privileges on a target from a role.

use smallvec::SmallVec;

use crate::Privilege;
use crate::target::{Symbol, Target};

/// `Revoke` operation. Named `RevokeV2` to avoid colliding with the v1
/// [`crate::Revoke`] payload during the migration window.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RevokeV2 {
    pub privileges: SmallVec<[Privilege; 2]>,
    pub target: Target,
    pub roles: SmallVec<[Symbol; 1]>,
    pub cascade: bool,
}
