//! Typed [`ExtensionPayload`] wrappers for `dol-stream`.
//!
//! `dol-stream` exposes three streaming verbs through the
//! [`Operation::Extension`](dol_ir::Operation::Extension) seam, each as its
//! own typed payload:
//!
//! | Identifier               | Payload struct            | Carries        |
//! |--------------------------|---------------------------|----------------|
//! | `dol.stream/window`      | [`WindowPayload`]         | [`WindowSpec`](crate::WindowSpec) |
//! | `dol.stream/timeseries`  | [`TimeSeriesPayload`]     | [`TimeSeriesOp`](crate::TimeSeriesOp) |
//! | `dol.stream/iot.sample`  | [`SamplePayload`]         | [`Sample`](crate::Sample) |
//!
//! Each payload registers its [`Symbol`] through [`register_window`],
//! [`register_timeseries`], and [`register_iot_sample`] respectively. Call
//! the appropriate `register_*` once per process before encoding/decoding,
//! passing the interned id of the corresponding extension name.

extern crate alloc;
use alloc::vec::Vec;

use core::sync::atomic::{AtomicU32, Ordering};

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

static WINDOW_SYMBOL: AtomicU32 = AtomicU32::new(0);
static TIMESERIES_SYMBOL: AtomicU32 = AtomicU32::new(0);
static IOT_SAMPLE_SYMBOL: AtomicU32 = AtomicU32::new(0);

/// Register the symbol that identifies the window-extension payload.
pub fn register_window(symbol: Symbol) {
    WINDOW_SYMBOL.store(symbol.0, Ordering::Relaxed);
}
/// Register the symbol that identifies the time-series-extension payload.
pub fn register_timeseries(symbol: Symbol) {
    TIMESERIES_SYMBOL.store(symbol.0, Ordering::Relaxed);
}
/// Register the symbol that identifies the IoT-sample extension payload.
pub fn register_iot_sample(symbol: Symbol) {
    IOT_SAMPLE_SYMBOL.store(symbol.0, Ordering::Relaxed);
}

macro_rules! impl_payload {
    ($payload:ident, $inner:path, $field:ident, $sym_static:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Debug, PartialEq)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $payload {
            /// Wrapped extension body.
            pub $field: $inner,
        }

        impl ExtensionPayload for $payload {
            fn extension_id() -> ExtensionId {
                ExtensionId::new(
                    Symbol::new($sym_static.load(Ordering::Relaxed)),
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
                    postcard::from_bytes(bytes).map_err(|_| concat!(stringify!($payload), " decode failed"))
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
                Operation::Extension(alloc::boxed::Box::new(
                    OperationExtension::from_payload(&self),
                ))
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
