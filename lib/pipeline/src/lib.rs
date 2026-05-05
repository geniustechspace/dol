//! # `dol-pipeline` — declarative dataflow IR
//!
//! A pipeline is a typed DAG of nodes joined by typed edges:
//!
//!   `Source`    feeds rows into the graph
//!   `Transform` rewrites rows (any relational/scalar op expressible in
//!               `dol-expr`, plus higher-order ops like `unnest`, `pivot`,
//!               `unpivot`, `gap_fill`, `asof_join`, `tdigest`, `approx_*`)
//!   `Sink`      consumes rows out of the graph
//!
//! Pipelines are **descriptive only** — this crate provides type inference
//! over their node schemas; execution is a backend concern.
//!
//! Pipelines compose with [`dol_ir::Operation`] via the
//! [`dol_ir::Operation::Extension`] seam: a [`Graph`] is wrapped in a
//! [`PipelinePayload`] (a typed
//! [`ExtensionPayload`](dol_ir::operation::ExtensionPayload)) and embedded
//! into a [`dol_ir::Program`].
//!
//! ## Status
//!
//! This crate currently models the IR; schema inference is intentionally
//! conservative (passes through known field lists) and is expected to grow
//! alongside `dol-check`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(clippy::large_enum_variant)]

pub mod extension;
pub mod graph;
pub mod node;
pub mod schema;

pub use extension::{EXTENSION_NAME, EXTENSION_SYMBOL, EXTENSION_VERSION, PipelinePayload};
pub use graph::{Graph, NodeIdx};
pub use node::{Node, Sink, Source, Transform};
pub use schema::{ColumnSchema, RowSchema};
