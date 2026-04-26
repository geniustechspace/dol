//! # `dol-stream` — streaming / time-series / IoT IR
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
//! These types are pure data; they hook into [`dol_ir::Statement`] via the
//! `Statement::Extension` seam and are validated by `dol-check`.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod iot;
pub mod timeseries;
pub mod window;

pub use iot::{Actuator, PayloadCodec, QoS, RetentionPolicy, Sample, Sensor, StoreAndForward};
pub use timeseries::{TimeSeriesOp, TimeUnit};
pub use window::{Trigger, Watermark, WindowSpec};
