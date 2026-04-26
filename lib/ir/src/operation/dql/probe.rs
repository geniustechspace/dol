//! `Probe` — existence / metadata-only check (HTTP `HEAD`, S3 `HeadObject`,
//! `EXISTS` predicate, file `stat`).

use crate::target::Target;

/// `Probe` operation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Probe {
    pub target: Target,
    /// When `true`, the probe should also report counts / sizes; otherwise
    /// existence-only.
    pub include_metadata: bool,
}
