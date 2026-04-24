//! # DOL — umbrella facade crate
//!
//! Re-exports the individual `dol-*` crates behind cargo feature flags so
//! downstream consumers can pick the slice of the stack they need without
//! pulling in the full dependency tree.
//!
//! ## Feature flags
//!
//! | Feature | Pulls in | Purpose |
//! |---------|----------|---------|
//! | *(default)* | `dol-types` only | Leaf primitives: `Value`, `Literal`, `DataType`, scalar wrappers. |
//! | `expr`   | `dol-expr`           | Expression arena + tree DSL. |
//! | `schema` | `dol-schema`         | `Entity`, `Field`, constraint types. |
//! | `ir`     | `dol-ir` (+expr+schema) | `Statement` enum, `Backend` trait, DDL/DML nodes. |
//! | `query`  | `dol-query` (+ir)    | User-facing builder API. |
//! | `full`   | all of the above     | Convenience aggregator. |
//! | `serde`  | (orthogonal)         | Turns on `serde` on every active sub-crate. |
//!
//! ## Curated surface
//!
//! Each layer is exposed via a re-exported sub-module rather than a flat
//! `pub use *::*`. To reach the stable, "blessed" surface of a layer use the
//! `prelude` of that layer:
//!
//! ```ignore
//! use dol::types::prelude::*;
//! # #[cfg(feature = "schema")]
//! use dol::schema::prelude::*;
//! ```
//!
//! For convenience this crate also exposes a top-level [`prelude`] that
//! globs every active layer's prelude.

#![deny(unsafe_code)]

pub use dol_types as types;

#[cfg(feature = "expr")]
pub use dol_expr as expr;

#[cfg(feature = "schema")]
pub use dol_schema as schema;

#[cfg(feature = "ir")]
pub use dol_ir as ir;

#[cfg(feature = "query")]
pub use dol_query as query;

/// Curated re-export of the most commonly used items from every active layer.
pub mod prelude {
    pub use crate::types::prelude::*;

    #[cfg(feature = "expr")]
    pub use crate::expr::prelude::*;

    #[cfg(feature = "schema")]
    pub use crate::schema::prelude::*;

    #[cfg(feature = "ir")]
    pub use crate::ir::prelude::*;

    #[cfg(feature = "query")]
    pub use crate::query::prelude::*;
}
