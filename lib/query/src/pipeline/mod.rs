//! Declarative dataflow IR (formerly `dol-pipeline`).
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
//! `dol_command::program::Program`. Hosting the payload wrapper on the
//! `dol-command` side preserves the v2 dependency graph
//! (`command → query`, never the reverse).

#![allow(clippy::large_enum_variant)]

pub mod graph;
pub mod node;
pub mod schema;

pub use graph::{Graph, NodeIdx};
pub use node::{Node, Sink, Source, Transform};
pub use schema::{ColumnSchema, RowSchema};
