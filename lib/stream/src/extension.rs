//! Typed [`ExtensionPayload`] wrappers for `dol-stream`.
//!
//! `dol-stream` exposes three streaming verbs through the
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

use dol_ir::operation::{ExtensionId, ExtensionPayload, OperationExtension};
use dol_ir::{Operation, Symbol};

use crate::{Sample, TimeSeriesOp, WindowSpec};

/// Stable extension name for the windowing payload.
pub const WINDOW_NAME: &str = "dol.stream/window";
/// Stable extension name for the time-series payload.
pub const TIMESERIES_NAME: &str = "dol.stream/timeseries";
/// Stable extension name for the IoT-sample payload.
pub const IOT_SAMPLE_NAME: &str = "dol.stream/iot.sample";
/// Wire-format version shared by all stream extension payloads.
pub const EXTENSION_VERSION: u32 = 1;

/// Stable [`Symbol`] identifying [`WindowPayload`] on the wire.
pub const WINDOW_SYMBOL: Symbol = Symbol::new(fnv1a_32(WINDOW_NAME.as_bytes()));
/// Stable [`Symbol`] identifying [`TimeSeriesPayload`] on the wire.
pub const TIMESERIES_SYMBOL: Symbol = Symbol::new(fnv1a_32(TIMESERIES_NAME.as_bytes()));
/// Stable [`Symbol`] identifying [`SamplePayload`] on the wire.
pub const IOT_SAMPLE_SYMBOL: Symbol = Symbol::new(fnv1a_32(IOT_SAMPLE_NAME.as_bytes()));

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

macro_rules! impl_payload {
    ($payload:ident, $inner:path, $field:ident, $sym_const:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Debug, PartialEq)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
                    // trees we use). Fail loudly rather than silently
                    // emitting an empty payload that decodes to defaults.
                    postcard::to_allocvec(self)
                        .expect(concat!(stringify!($payload), " encode failed"))
                }
                #[cfg(not(feature = "serde"))]
                {
                    Vec::new()
                }
            }

            fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
                #[cfg(feature = "serde")]
                {
                    postcard::from_bytes(bytes)
                        .map_err(|_| concat!(stringify!($payload), " decode failed"))
                }
                #[cfg(not(feature = "serde"))]
                {
                    let _ = bytes;
                    Err("dol-stream serde feature not enabled")
                }
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
