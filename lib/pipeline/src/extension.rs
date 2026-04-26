//! Typed [`ExtensionPayload`] wrapper for `dol-pipeline`.
//!
//! Wraps a [`Graph`](crate::Graph) into an [`OperationExtension`] under the
//! stable identifier `dol.pipeline/graph` (version 1). Encoded with
//! `postcard` when the `serde` feature is enabled; otherwise [`encode`]
//! returns a placeholder error payload.

extern crate alloc;
use alloc::vec::Vec;

use core::sync::atomic::{AtomicU32, Ordering};

use dol_ir::operation::{ExtensionId, ExtensionPayload};
use dol_ir::{Operation, Symbol};

use crate::Graph;

/// Stable extension name.
pub const EXTENSION_NAME: &str = "dol.pipeline/graph";
/// Wire-format version.
pub const EXTENSION_VERSION: u32 = 1;

// Symbols are produced by the `Interner` in `dol-expr`, so we cannot
// allocate one at compile time. Instead, the registering crate resolves the
// symbol once per program through `register` below — `decode`/`encode`
// callers should *not* read this until the registration has happened. The
// default value of `0` is the same as `Symbol::default()`.
static REGISTERED_SYMBOL: AtomicU32 = AtomicU32::new(0);

/// Register the symbol that will identify this extension. Call from
/// `Program` setup, passing the interned id of [`EXTENSION_NAME`].
pub fn register(symbol: Symbol) {
    REGISTERED_SYMBOL.store(symbol.0, Ordering::Relaxed);
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
        ExtensionId::new(
            Symbol::new(REGISTERED_SYMBOL.load(Ordering::Relaxed)),
            EXTENSION_VERSION,
        )
    }

    fn encode(&self) -> Vec<u8> {
        #[cfg(feature = "serde")]
        {
            postcard::to_allocvec(self).unwrap_or_default()
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
