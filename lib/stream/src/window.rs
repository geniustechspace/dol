//! Window specifications.

use dol_expr::ids::NodeId;

/// Streaming window specification.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum WindowSpec {
    /// Non-overlapping fixed-duration windows.
    Tumbling {
        /// Time column.
        time: NodeId,
        /// Window size.
        size_ms: u64,
    },
    /// Overlapping fixed-duration windows.
    Hopping {
        /// Time column.
        time: NodeId,
        /// Window size.
        size_ms: u64,
        /// Hop interval.
        hop_ms: u64,
    },
    /// Variable-duration windows that close after a gap of inactivity.
    Session {
        /// Time column.
        time: NodeId,
        /// Inactivity gap that closes the window.
        gap_ms: u64,
    },
    /// Windows defined by row count rather than time.
    CountBased {
        /// Number of rows per window.
        rows: u32,
    },
}

/// Watermark policy for late-data tolerance.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Watermark {
    /// Time column the watermark advances on.
    pub time: NodeId,
    /// How far behind real time the watermark is allowed to lag.
    pub max_out_of_order_ms: u64,
    /// Total tolerated lateness past the window close.
    pub allowed_lateness_ms: u64,
}

/// When window results are emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Trigger {
    /// Fire once when the watermark passes the window's end.
    AfterWatermark,
    /// Fire after every record.
    PerRecord,
    /// Fire periodically while the window is open.
    Periodic {
        /// Inter-fire period in milliseconds.
        period_ms: u64,
    },
    /// Fire when an explicit signal is received from the runtime.
    OnSignal,
}
