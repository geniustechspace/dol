//! `Extension` — typed open-ended extension payload.
//!
//! Higher-level crates (notably `dol-stream` and `dol-pipeline`) attach new
//! verbs via `Extension` rather than extending the closed `Operation` enum,
//! so streaming/IoT vocabulary can evolve independently of the core.

use crate::target::Symbol;

/// Stable extension identifier.
///
/// `id` is interned so equality is a single integer compare.
/// `version` tracks the wire-format revision of the extension's payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExtensionId {
    pub name: Symbol,
    pub version: u32,
}

impl ExtensionId {
    #[inline]
    pub const fn new(name: Symbol, version: u32) -> Self {
        Self { name, version }
    }
}

/// Open extension payload.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OperationExtension {
    pub id: ExtensionId,
    /// Codec-encoded body understood by the registering crate.
    pub payload: Vec<u8>,
}
