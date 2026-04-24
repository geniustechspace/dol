//! Re-export of the unified type descriptor from `dol-types`.
//!
//! The old `FieldType` enum has been replaced by [`DataType`] from the
//! `dol-types` module, which provides ~44 variants covering primitives,
//! composites, temporal, network, geometric, and semantic types.

pub use dol_types::DataType;
