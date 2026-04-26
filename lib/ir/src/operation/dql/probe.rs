//! `Probe` — existence / metadata-only check (HTTP `HEAD`, S3 `HeadObject`,
//! `EXISTS` predicate, file `stat`).

use crate::target::Target;

/// `Probe` operation.
///
/// Existence / metadata-only check (HTTP HEAD, S3 HeadObject, file stat).
///
/// # Examples
///
/// ```
/// use dol_ir::operation::Probe;
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::Operation;
///
/// // HEAD s3://artifacts/build.log
/// let op: Operation = Probe {
///     target: Target::new(TargetKind::Blob, Locator::new(Symbol::new(0))),
///     include_metadata: true,
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::OpKind::Probe);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Probe {
    /// Target to probe.
    pub target: Target,
    /// When `true`, the probe should also report counts / sizes; otherwise
    /// existence-only.
    pub include_metadata: bool,
}
