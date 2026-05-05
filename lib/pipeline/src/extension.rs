//! Typed [`ExtensionPayload`] wrapper for `dol-pipeline`.
//!
//! Wraps a [`Graph`] into an [`OperationExtension`](dol_ir::operation::OperationExtension)
//! under the stable identifier `dol.pipeline/graph` (version 1). Encoded
//! with `postcard` when the `serde` feature is enabled; otherwise
//! [`PipelinePayload::encode`] returns an empty byte vector and
//! [`PipelinePayload::decode`] reports a feature-gated error.

extern crate alloc;
use alloc::vec::Vec;

use dol_ir::operation::{ExtensionId, ExtensionPayload};
use dol_ir::{Operation, Symbol};

use crate::Graph;

/// Stable extension name.
pub const EXTENSION_NAME: &str = "dol.pipeline/graph";
/// Wire-format version.
pub const EXTENSION_VERSION: u32 = 1;

/// Stable [`Symbol`] identifying this extension on the wire.
///
/// Derived deterministically from [`EXTENSION_NAME`] via the FNV-1a 32-bit
/// hash so that every process — regardless of the [`Interner`](dol_expr::Interner)
/// used to build the surrounding `Program` — produces the same id. This
/// eliminates the previous design's process-global `AtomicU32`, which was
/// brittle across `Program`s built or decoded with different interners.
pub const EXTENSION_SYMBOL: Symbol = Symbol::from_hash(fnv1a_32(EXTENSION_NAME.as_bytes()));

/// `const`-eval FNV-1a 32-bit hash. Stable; matches the spec basis/prime.
const fn fnv1a_32(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u32;
        hash = hash.wrapping_mul(0x01000193);
        i += 1;
    }
    hash
}

/// Typed pipeline payload — a [`Graph`] embedded in an `Operation::Extension`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PipelinePayload {
    /// The wrapped pipeline graph.
    pub graph: Graph,
}

impl ExtensionPayload for PipelinePayload {
    fn extension_id() -> ExtensionId {
        ExtensionId::new(EXTENSION_SYMBOL, EXTENSION_VERSION)
    }

    fn encode(&self) -> Vec<u8> {
        #[cfg(feature = "serde")]
        {
            // `postcard::to_allocvec` only fails for shapes postcard cannot
            // represent (cycles, non-finite floats with custom serializers).
            // `PipelinePayload` is a simple owned tree, so a failure here
            // is a structural bug — fail loudly rather than silently
            // producing an empty payload that decodes to defaults.
            postcard::to_allocvec(self).expect("PipelinePayload encode failed")
        }
        #[cfg(not(feature = "serde"))]
        {
            Vec::new()
        }
    }

    fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        #[cfg(feature = "serde")]
        {
            postcard::from_bytes(bytes).map_err(|_| "PipelinePayload decode failed")
        }
        #[cfg(not(feature = "serde"))]
        {
            let _ = bytes;
            Err("dol-pipeline serde feature not enabled")
        }
    }
}

impl PipelinePayload {
    /// Wrap `graph` into an [`Operation::Extension`].
    pub fn into_operation(self) -> Operation {
        Operation::Extension(alloc::boxed::Box::new(
            dol_ir::operation::OperationExtension::from_payload(&self),
        ))
    }
}
