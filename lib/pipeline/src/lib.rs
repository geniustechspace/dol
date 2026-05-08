//! Declarative dataflow IR (formerly `dol_query::pipeline`).
//!
//! A pipeline is a typed DAG of nodes joined by typed edges:
//!
//!   `Source`    feeds rows into the graph
//!   `Transform` rewrites rows (any relational/scalar op expressible in
//!               `dol-expr`, plus higher-order ops like `unnest`, `pivot`,
//!               `unpivot`, `gap_fill`, `asof_join`, `tdigest`, `approx_*`)
//!   `Sink`      consumes rows out of the graph
//!
//! Pipelines are **descriptive only** — this module provides type inference
//! over their node schemas; execution is a backend concern.
//!
//! Pipelines compose with `dol_command::operation::Operation` via the
//! `Operation::Extension` seam: a [`Graph`] is wrapped in a
//! `dol_command::query_extensions::pipeline::PipelinePayload` (a typed
//! `dol_command::operation::ExtensionPayload`) and embedded into a
//! `dol_command::program::Program`.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
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
#![allow(clippy::large_enum_variant)]

pub mod graph;
pub mod node;
pub mod schema;

pub use graph::{Graph, NodeIdx};
pub use node::{Node, Sink, Source, Transform};
pub use schema::{ColumnSchema, RowSchema};
