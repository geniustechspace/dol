//! Time-series operators.

use dol_expr::ids::NodeId;

/// Calendar / wall-clock time unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum TimeUnit {
    /// Milliseconds.
    Millisecond,
    /// Seconds.
    Second,
    /// Minutes.
    Minute,
    /// Hours.
    Hour,
    /// Days.
    Day,
    /// Weeks.
    Week,
    /// Months.
    Month,
    /// Years.
    Year,
}

/// A time-series operator referenced from a pipeline transform.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum TimeSeriesOp {
    /// Bucket rows by `(time / size unit) * size`.
    TimeBucket {
        /// Time column.
        time: NodeId,
        /// Bucket size.
        size: u64,
        /// Bucket unit.
        unit: TimeUnit,
    },
    /// Reduce sample density by `factor` (drop every (factor - 1)/factor row).
    Downsample {
        /// Time column.
        time: NodeId,
        /// Reduction factor.
        factor: u32,
    },
    /// Insert rows for missing buckets.
    GapFill {
        /// Time column.
        time: NodeId,
        /// Bucket size.
        size: u64,
        /// Bucket unit.
        unit: TimeUnit,
    },
    /// Last-observation-carried-forward fill.
    Locf {
        /// Column to fill.
        column: NodeId,
    },
    /// Per-bucket derivative.
    Rate {
        /// Numeric column.
        column: NodeId,
        /// Time column.
        time: NodeId,
    },
    /// Per-row delta (`x[n] - x[n-1]`).
    Delta {
        /// Numeric column.
        column: NodeId,
    },
}
