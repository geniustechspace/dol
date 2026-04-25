//! Telemetry / IoT vocabulary.
//!
//! These types model edge-device intent at the IR layer. Backends turn them
//! into concrete connectors (MQTT, OPC-UA, …); none of that lives here.

use dol_types::DataType;

/// A device that produces samples.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sensor {
    /// Stable sensor identifier.
    pub id: alloc::string::String,
    /// Reading data type.
    pub reading_type: DataType,
    /// Optional unit of measurement label (e.g. `"°C"`, `"hPa"`).
    pub unit: Option<alloc::string::String>,
    /// Nominal sample interval in milliseconds.
    pub sample_interval_ms: u64,
}

/// A device that accepts commands.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Actuator {
    /// Stable actuator identifier.
    pub id: alloc::string::String,
    /// Command payload type.
    pub command_type: DataType,
}

/// A single sensor reading.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sample {
    /// Producing sensor id.
    pub sensor_id: alloc::string::String,
    /// Wall-clock timestamp (ms since the unix epoch).
    pub timestamp_ms: i64,
    /// The reading itself, encoded as the canonical wire form.
    pub value: alloc::vec::Vec<u8>,
}

/// MQTT-style quality of service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QoS {
    /// Fire-and-forget.
    AtMostOnce,
    /// Re-send until acked, may duplicate.
    AtLeastOnce,
    /// Exactly-once delivery.
    ExactlyOnce,
}

/// Retention / compaction policy for time-series storage.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RetentionPolicy {
    /// Maximum age before rows become eligible for compaction.
    pub keep_for_ms: u64,
    /// Optional aggregation hint (`"avg"`, `"max"`, …) used during compaction.
    pub compact_with: Option<alloc::string::String>,
}

/// Edge-device store-and-forward intent.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StoreAndForward {
    /// Maximum buffered bytes before back-pressure kicks in.
    pub buffer_bytes: u64,
    /// Drop oldest rows on overflow rather than block.
    pub drop_oldest_on_overflow: bool,
    /// Re-send batch size.
    pub batch_size: u32,
}

/// On-the-wire payload codec hint. Implementation lives in connectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PayloadCodec {
    /// `dol-wire`'s postcard form.
    Postcard,
    /// CBOR.
    Cbor,
    /// Protobuf.
    Protobuf,
    /// JSON.
    Json,
}

extern crate alloc;
