//! `Extension` — typed open-ended extension payload.
//!
//! Higher-level crates (notably `dol-stream` and `dol-pipeline`) attach new
//! verbs via `Extension` rather than extending the closed `Operation` enum,
//! so streaming/IoT vocabulary can evolve independently of the core.

extern crate alloc;
use alloc::vec::Vec;

use crate::target::Symbol;

/// Stable extension identifier.
///
/// `name` is interned so equality is a single integer compare. `version`
/// tracks the wire-format revision of the extension's payload, allowing
/// crates to evolve the body without changing the identifier.
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

/// Typed extension payload trait.
///
/// Higher-level crates implement [`ExtensionPayload`] for their own typed
/// payloads (e.g. `dol_pipeline::PipelinePayload`,
/// `dol_stream::WindowPayload`) and use [`OperationExtension::from_payload`]
/// to encode them into the open `OperationExtension` body.
///
/// The trait deliberately keeps the wire encoding pluggable — implementers
/// can choose `postcard`, JSON, or any other codec, as long as `encode` and
/// `decode` round-trip.
pub trait ExtensionPayload: Sized {
    /// The stable [`ExtensionId`] this payload registers under.
    ///
    /// Implementers should hold a `OnceLock<ExtensionId>` (or equivalent)
    /// so the [`Symbol`] is interned exactly once per process.
    fn extension_id() -> ExtensionId;

    /// Encode `self` into the [`OperationExtension::payload`] byte buffer.
    fn encode(&self) -> Vec<u8>;

    /// Decode a [`OperationExtension::payload`] byte buffer into `Self`.
    ///
    /// Returns an opaque `&'static str` error rather than introducing a
    /// crate-wide error type — callers (typically `dol-check`) lift this
    /// into a [`dol_core::Diagnostic`].
    fn decode(bytes: &[u8]) -> Result<Self, &'static str>;
}

impl OperationExtension {
    /// Build an [`OperationExtension`] from any [`ExtensionPayload`] impl.
    pub fn from_payload<P: ExtensionPayload>(payload: &P) -> Self {
        Self {
            id: P::extension_id(),
            payload: payload.encode(),
        }
    }

    /// Try to decode the body as `P`. Returns `Err` when the [`ExtensionId`]
    /// (name + version) doesn't match the registered one for `P` or when
    /// `P::decode` fails.
    pub fn decode_as<P: ExtensionPayload>(&self) -> Result<P, &'static str> {
        if self.id != P::extension_id() {
            return Err("ExtensionId mismatch");
        }
        P::decode(&self.payload)
    }
}
