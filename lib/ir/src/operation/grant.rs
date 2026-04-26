//! `Grant` — bestow privileges on a target to a role.
//!
//! v2 generalises the v1 [`crate::Privilege`] over any `TargetKind`. The
//! [`crate::Privilege`] enum is reused as-is.

use smallvec::SmallVec;

use crate::Privilege;
use crate::target::{Symbol, Target};

/// `Grant` operation. Named `GrantV2` to avoid colliding with the v1
/// [`crate::Grant`] payload during the migration window.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GrantV2 {
    pub privileges: SmallVec<[Privilege; 2]>,
    pub target: Target,
    pub roles: SmallVec<[Symbol; 1]>,
    pub with_grant_option: bool,
}
