//! Typed [`OperationExtension`](crate::operation::OperationExtension)
//! wrappers for [`dol_query`]'s streaming and pipeline data types.
//!
//! These wrappers were previously hosted inside `dol-query`'s own
//! `stream::extension` and `pipeline::extension` modules. Hosting them
//! there forced `dol-query` to depend on `dol-command` so it could `impl
//! ExtensionPayload` and call `OperationExtension::from_payload`. The
//! payloads now live here in `dol-command`, behind the `query` feature
//! (default-on), preserving the public symbols on the wire while
//! restoring the documented `command → query` dependency direction.
//!
//! - [`stream::WindowPayload`] / [`stream::TimeSeriesPayload`] /
//!   [`stream::SamplePayload`] wrap the matching `dol_query::stream`
//!   data types.
//! - [`pipeline::PipelinePayload`] wraps a `dol_query::pipeline::Graph`.
//!
//! All [`crate::target::Symbol`] identifiers (`WINDOW_SYMBOL`,
//! `TIMESERIES_SYMBOL`, `IOT_SAMPLE_SYMBOL`, `EXTENSION_SYMBOL`) are
//! deterministic FNV-1a 32-bit hashes of their stable extension names,
//! so they are wire-stable across processes regardless of which
//! [`Interner`](dol_expr::Interner) built the surrounding `Program`.

pub mod pipeline;
pub mod stream;
