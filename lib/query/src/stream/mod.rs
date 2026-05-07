//! Streaming / time-series / IoT IR (formerly `dol-stream`).
//!
//! Extends the core IR with a handful of operators that don't fit cleanly
//! into batch SQL semantics:
//!
//! - **Windows**: tumbling, hopping, session, count-based.
//! - **Watermarks** with allowed lateness and triggers.
//! - **Time-series ops**: `time_bucket`, `downsample`, `gap_fill`, `locf`,
//!   `rate`, `delta`.
//! - **Telemetry / IoT vocabulary**: sensors, actuators, samples, retention
//!   policies, store-and-forward intent, QoS hints, payload codecs.
//!
//! These types are pure data; they hook into
//! `dol_command::operation::Operation` via the `Operation::Extension`
//! seam through the typed payloads in
//! `dol_command::query_extensions::stream` (`WindowPayload`,
//! `TimeSeriesPayload`, `SamplePayload`) and are validated by `dol-check`.
//! Hosting the payload wrappers on the `dol-command` side preserves the
//! v2 dependency graph (`command → query`, never the reverse).

pub mod iot;
pub mod timeseries;
pub mod window;

pub use iot::{Actuator, PayloadCodec, QoS, RetentionPolicy, Sample, Sensor, StoreAndForward};
pub use timeseries::{TimeSeriesOp, TimeUnit};
pub use window::{Trigger, Watermark, WindowSpec};
