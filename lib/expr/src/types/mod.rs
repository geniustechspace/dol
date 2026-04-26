//! DOL type system — re-exported from `dol-core`.
//!
//! The canonical implementation lives in the `dol-core` leaf crate.
//! All paths remain backwards-compatible: `dol_expr::types::DataType` etc.
//! still resolve correctly.

pub use dol_core::*;

// Re-export sub-modules for path compatibility.
pub use dol_core::{binary, datetime, descriptor, error, geo, network, numeric, value};
