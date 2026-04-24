//! DOL type system — re-exported from `dol-types`.
//!
//! The canonical implementation lives in the `dol-types` leaf crate.
//! All paths remain backwards-compatible: `dol_expr::types::DataType` etc.
//! still resolve correctly.

pub use dol_types::*;

// Re-export sub-modules for path compatibility.
pub use dol_types::{binary, datetime, descriptor, error, geo, network, numeric, value};
