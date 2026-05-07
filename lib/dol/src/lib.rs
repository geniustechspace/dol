//! # DOL — umbrella facade crate
//!
//! Re-exports the individual `dol-*` crates behind cargo feature flags so
//! downstream consumers can pick the slice of the stack they need without
//! pulling in the full dependency tree.
//!
//! ## Layered surface
//!
//! | Feature      | Pulls in                       | Purpose                                                    |
//! |--------------|--------------------------------|------------------------------------------------------------|
//! | *(default)*  | `dol-core`                     | Spans, diagnostics, and the value/type system (`Value`, `Literal`, `DataType`, …). |
//! | `expr`       | `dol-expr`                     | Expression arena + tree DSL.                               |
//! | `schema`     | `dol-schema`                   | Entities, fields, constraints, relations, lookups, policies. |
//! | `ir`         | `dol-ir`                       | `Operation`, `Program`, `Backend`, `BackendCapabilities`.  |
//! | `wire`       | `dol-wire`                     | Canonical wire envelope + postcard / JSON codec helpers.   |
//! | `check`      | `dol-check`                    | Static validator (type / schema / capability / lint).      |
//! | `fmt`        | `dol-fmt`                      | Canonical pretty-printer.                                  |
//! | `query`      | `dol-query`                    | Fluent builder DSL, plus the streaming / pipeline / IoT IR (formerly the separate `dol-stream` and `dol-pipeline` crates). |
//!
//! ## Curated presets
//!
//! - `core`    — `expr + schema + ir + query`.
//! - `full`    — every layer DOL ships.
//! - `iot-min` — minimal IoT-edge slice (core + IR + query + postcard wire).
//!
//! ## Universal serde
//!
//! Adding `serde` turns on the `serde` feature on every active sub-crate.

#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )
)]
#![warn(missing_docs)]

pub use dol_core as core;

#[cfg(feature = "expr")]
pub use dol_expr as expr;

#[cfg(feature = "schema")]
pub use dol_schema as schema;

#[cfg(feature = "ir")]
pub use dol_ir as ir;

#[cfg(feature = "wire")]
pub use dol_wire as wire;

#[cfg(feature = "check")]
pub use dol_check as check;

#[cfg(feature = "fmt")]
pub use dol_fmt as fmt;

#[cfg(feature = "query")]
pub use dol_query as query;

/// Curated re-export of the most commonly used items from every active layer.
pub mod prelude {
    pub use crate::core::prelude::*;

    #[cfg(feature = "expr")]
    pub use crate::expr::prelude::*;

    #[cfg(feature = "schema")]
    pub use crate::schema::prelude::*;

    #[cfg(feature = "ir")]
    pub use crate::ir::prelude::*;

    #[cfg(feature = "query")]
    pub use crate::query::prelude::*;
}
