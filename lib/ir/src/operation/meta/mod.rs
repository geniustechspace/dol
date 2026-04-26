//! Meta operation payloads — escape hatches that do not fit DDL/DML/DQL/ACL.
//!
//! - [`OperationExtension`] is the typed open-ended extension envelope used
//!   by `dol-pipeline` and `dol-stream` to ship payloads (window specs,
//!   pipeline graphs, IoT samples) without growing the closed `Operation`
//!   enum.
//! - [`RawOp`] (feature-gated) is a backend-specific raw passthrough;
//!   programs that use it acquire the `RAW_PASSTHROUGH` capability tag.

pub mod extension;
#[cfg(feature = "raw")]
pub mod raw;

pub use extension::{ExtensionId, ExtensionPayload, OperationExtension};
#[cfg(feature = "raw")]
pub use raw::RawOp;
