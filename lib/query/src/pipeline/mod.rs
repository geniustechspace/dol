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
//! Pipelines compose with [`dol_ir::operation::Operation`] via the
//! [`Operation::Extension`](dol_ir::operation::Operation::Extension) seam:
//! a [`Graph`] is wrapped in a [`PipelinePayload`] (a typed
//! [`ExtensionPayload`](dol_ir::operation::ExtensionPayload)) and embedded
//! into a [`dol_ir::program::Program`].

#![allow(clippy::large_enum_variant)]

pub mod extension;
pub mod graph;
pub mod node;
pub mod schema;

pub use extension::{EXTENSION_NAME, EXTENSION_SYMBOL, EXTENSION_VERSION, PipelinePayload};
pub use graph::{Graph, NodeIdx};
pub use node::{Node, Sink, Source, Transform};
pub use schema::{ColumnSchema, RowSchema};
