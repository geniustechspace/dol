//! `Raw` — feature-gated escape hatch for pre-built backend dialects.
//!
//! Available only with the `raw` feature. Use of `Raw` adds the
//! [`CapabilityTag::RAW_PASSTHROUGH`](crate::capabilities::CapabilityTag::RAW_PASSTHROUGH)
//! tag to the program's required capability set.
//!
//! # Placement decision (v2)
//!
//! `RawOp` lives in `dol-command` (this crate) **behind the `raw` feature** for
//! the 0.2.0 cut, rather than being moved to a hypothetical
//! `dol-backends-common`. Rationale:
//!
//! * The IR enum is closed; `Raw` participates in `Operation::kind()`,
//!   wire codec coverage, and capability-tag accounting alongside the
//!   structured verbs. Splitting it out would require a second open
//!   variant (`Operation::Backend(Box<dyn Any>)`) which loses the
//!   compile-time exhaustiveness check.
//! * No backend crate exists yet (workspace globs `backends/*` are
//!   commented out until the first backend lands), so there is no
//!   crate to move it to without prematurely creating one.
//! * Downstream consumers that do not want the escape hatch leave the
//!   feature off; the variant disappears from the enum entirely.
//!
//! # Capability-tag invariant
//!
//! Every program containing an `Operation::Raw(_)` **must** carry the
//! [`RAW_PASSTHROUGH`](crate::capabilities::CapabilityTag::RAW_PASSTHROUGH)
//! capability tag. This invariant is checked when capabilities are
//! computed for a program; a program asserting fewer capabilities than
//! its operations require fails validation. Backends that do not
//! advertise `RAW_PASSTHROUGH` reject programs that contain `Raw`.

use alloc::string::String;
use dol_expr::ids::NodeId;
use smallvec::SmallVec;

use crate::target::Symbol;

/// Pre-built dialect-specific body.
///
/// Escape hatch for passing through raw SQL or other dialect-specific
/// statements. Requires the `raw` feature and adds the
/// [`RAW_PASSTHROUGH`](crate::capabilities::CapabilityTag::RAW_PASSTHROUGH)
/// capability requirement.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RawOp {
    /// Optional dialect tag (`postgres`, `mongo`, `redis`, …).
    pub dialect: Option<Symbol>,
    /// Verbatim body string in the chosen dialect.
    pub body: String,
    /// Arena `NodeId`s for placeholder bindings referenced from `body`.
    pub params: SmallVec<[NodeId; 4]>,
}
