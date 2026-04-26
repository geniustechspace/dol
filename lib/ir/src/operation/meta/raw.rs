//! `Raw` — feature-gated escape hatch for pre-built backend dialects.
//!
//! Available only with the `raw` feature. Use of `Raw` adds the
//! [`CapabilityTag::RawPassthrough`](crate::capabilities::CapabilityTag) tag
//! to the program's required capability set.

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RawOp {
    /// Optional dialect tag (`postgres`, `mongo`, `redis`, …).
    pub dialect: Option<Symbol>,
    /// Verbatim body string in the chosen dialect.
    pub body: String,
    /// Arena `NodeId`s for placeholder bindings referenced from `body`.
    pub params: SmallVec<[NodeId; 4]>,
}
