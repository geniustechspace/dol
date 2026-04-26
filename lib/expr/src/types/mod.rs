//! DOL type system — re-exported from `dol-core`.
//!
//! The canonical implementation lives in the `dol-core` leaf crate.
//! All paths remain backwards-compatible: `dol_expr::types::DataType` etc.
//! still resolve correctly.

pub use dol_core::*;

// Re-export sub-modules for path compatibility. Granular modules are
// feature-gated mirroring `dol-core`'s default-on flags.
#[cfg(feature = "datetime")]
pub use dol_core::datetime;
#[cfg(feature = "geo")]
pub use dol_core::geo;
#[cfg(feature = "network")]
pub use dol_core::network;
#[cfg(feature = "numeric")]
pub use dol_core::numeric;
pub use dol_core::{binary, descriptor, error, value};
