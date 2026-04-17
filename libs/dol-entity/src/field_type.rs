//! Re-export of the unified type descriptor from `dol-core::types`.
//!
//! The old `FieldType` enum has been replaced by [`DataType`] from the
//! `dol-core::types` module, which provides ~44 variants covering primitives,
//! composites, temporal, network, geometric, and semantic types.

pub use dol_core::types::DataType;
