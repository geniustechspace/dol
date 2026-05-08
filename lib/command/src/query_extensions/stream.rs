//! Typed [`ExtensionPayload`] wrappers for `dol_stream`.
//!
//! `dol_stream` exposes three streaming verbs through the
//! `Operation::Extension` seam, each as its own typed payload:
//!
//! | Identifier               | Payload struct            | Carries        |
//! |--------------------------|---------------------------|----------------|
//! | `dol.stream/window`      | [`WindowPayload`]         | [`WindowSpec`] |
//! | `dol.stream/timeseries`  | [`TimeSeriesPayload`]     | [`TimeSeriesOp`] |
//! | `dol.stream/iot.sample`  | [`SamplePayload`]         | [`Sample`] |
//!
//! Each payload's [`Symbol`] is a deterministic FNV-1a 32-bit hash of its
//! extension name, so the [`ExtensionId`] is stable across processes and
//! independent of any [`Interner`](dol_expr::Interner). This replaces the
//! earlier process-global `AtomicU32` registration model, which was brittle
//! when `Program`s were built or decoded with different interners.

extern crate alloc;
use alloc::vec::Vec;

use crate::operation::{ExtensionId, ExtensionPayload, Operation, OperationExtension};
use crate::target::Symbol;

use dol_stream::{Sample, TimeSeriesOp, WindowSpec};

/// Stable extension name for the windowing payload.
pub const WINDOW_NAME: &str = "dol.stream/window";
/// Stable extension name for the time-series payload.
pub const TIMESERIES_NAME: &str = "dol.stream/timeseries";
/// Stable extension name for the IoT-sample payload.
pub const IOT_SAMPLE_NAME: &str = "dol.stream/iot.sample";
/// Wire-format version shared by all stream extension payloads.
pub const EXTENSION_VERSION: u32 = 1;

/// Stable [`Symbol`] identifying [`WindowPayload`] on the wire.
pub const WINDOW_SYMBOL: Symbol = Symbol::from_hash(fnv1a_32(WINDOW_NAME.as_bytes()));
/// Stable [`Symbol`] identifying [`TimeSeriesPayload`] on the wire.
pub const TIMESERIES_SYMBOL: Symbol = Symbol::from_hash(fnv1a_32(TIMESERIES_NAME.as_bytes()));
/// Stable [`Symbol`] identifying [`SamplePayload`] on the wire.
pub const IOT_SAMPLE_SYMBOL: Symbol = Symbol::from_hash(fnv1a_32(IOT_SAMPLE_NAME.as_bytes()));

/// `const`-eval FNV-1a 32-bit hash. Stable; matches the spec basis/prime.
// `i < bytes.len()` bounds the indexing; `bytes.len() <= isize::MAX` so
// `i += 1` cannot overflow `usize`.
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
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

macro_rules! impl_payload {
    ($payload:ident, $inner:path, $field:ident, $sym_const:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Debug, PartialEq)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize))]
        pub struct $payload {
            /// Wrapped extension body.
            pub $field: $inner,
        }

        impl ExtensionPayload for $payload {
            fn extension_id() -> ExtensionId {
                ExtensionId::new($sym_const, EXTENSION_VERSION)
            }

            fn encode(&self) -> Vec<u8> {
                #[cfg(feature = "serde")]
                {
                    // Encoding failures here would indicate a structural
                    // bug in the payload type (postcard handles all owned
                    // trees we use). v2 forbids `panic!` in production
                    // code, so on error we emit the single-byte sentinel
                    // `0xFF` — an invalid postcard varint discriminant
                    // that the matching `decode` rejects with a clean
                    // codec error.
                    postcard::to_allocvec(self).unwrap_or_else(|_| alloc::vec![0xFFu8])
                }
                #[cfg(not(feature = "serde"))]
                {
                    Vec::new()
                }
            }

            fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
                // v2: in-memory IR/AST types (including extension
                // payloads) no longer implement `serde::Deserialize`.
                // The replacement is a budget-threaded
                // `dol-wire::Decode` impl, which has not yet landed for
                // these payloads. Until it does, decode is explicitly
                // unsupported.
                let _ = bytes;
                Err(concat!(
                    stringify!($payload),
                    " decode: v2 wire-in via dol-wire::Decode not yet implemented"
                ))
            }
        }

        impl $payload {
            /// Wrap into an [`Operation::Extension`].
            pub fn into_operation(self) -> Operation {
                Operation::Extension(alloc::boxed::Box::new(OperationExtension::from_payload(
                    &self,
                )))
            }
        }
    };
}

impl_payload!(
    WindowPayload,
    WindowSpec,
    spec,
    WINDOW_SYMBOL,
    "Typed payload wrapping a [`WindowSpec`] for `Operation::Extension`."
);
impl_payload!(
    TimeSeriesPayload,
    TimeSeriesOp,
    op,
    TIMESERIES_SYMBOL,
    "Typed payload wrapping a [`TimeSeriesOp`] for `Operation::Extension`."
);
impl_payload!(
    SamplePayload,
    Sample,
    sample,
    IOT_SAMPLE_SYMBOL,
    "Typed payload wrapping an IoT [`Sample`] for `Operation::Extension`."
);
