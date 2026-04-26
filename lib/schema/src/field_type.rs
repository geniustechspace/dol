//! Re-export of the unified type descriptor from `dol-core`.
//!
//! The old `FieldType` enum has been replaced by [`DataType`] from the
//! `dol-core` module, which provides ~44 variants covering primitives,
//! composites, temporal, network, geometric, and semantic types.

pub use dol_core::DataType;
