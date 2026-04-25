//! Pipeline node kinds.

use dol_expr::ids::NodeId;
use smallvec::SmallVec;

use crate::schema::RowSchema;

/// A node in the dataflow graph.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Node {
    /// Reads rows into the graph.
    Source(Source),
    /// Rewrites rows.
    Transform(Transform),
    /// Consumes rows out of the graph.
    Sink(Sink),
}

/// Where rows enter the graph.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Source {
    /// Read from a named entity (table, collection, …).
    Entity {
        /// Fully-qualified entity address (interned at the IR layer).
        target: alloc::string::String,
        /// Output schema contracted by the source (or empty if unknown).
        schema: RowSchema,
    },
    /// Read from a file at `path`.
    File {
        /// Backend-resolvable path.
        path: alloc::string::String,
        /// Hint at the file format (`"csv"`, `"parquet"`, …).
        format: alloc::string::String,
        /// Output schema contracted by the source.
        schema: RowSchema,
    },
    /// Read from an object store (e.g. S3).
    Object {
        /// `bucket/prefix`.
        location: alloc::string::String,
        /// Output schema contracted by the source.
        schema: RowSchema,
    },
    /// Inline parameter feed.
    Parameter {
        /// Name of the named parameter.
        name: alloc::string::String,
        /// Output schema.
        schema: RowSchema,
    },
}

/// In-graph row transformation.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Transform {
    /// Predicate filter expressed as a `dol-expr` node id.
    Filter {
        /// Predicate expression.
        predicate: NodeId,
    },
    /// Projection: a new ordered list of expressions becomes the row.
    Project {
        /// Output column expressions.
        exprs: SmallVec<[NodeId; 8]>,
    },
    /// Aggregation grouped by the given keys.
    Aggregate {
        /// Group-by keys.
        keys: SmallVec<[NodeId; 4]>,
        /// Aggregation functions over the group.
        aggs: SmallVec<[NodeId; 4]>,
    },
    /// Cross-row join.
    Join {
        /// Inner / left / right / full / cross.
        kind: JoinKind,
        /// Predicate.
        on: NodeId,
    },
    /// Unnest a collection-typed column into rows.
    Unnest {
        /// Column to unnest.
        column: NodeId,
    },
    /// Wide-to-long.
    Unpivot {
        /// Columns to keep as identifiers.
        id_columns: SmallVec<[NodeId; 4]>,
        /// Columns to unpivot into `(name, value)` rows.
        value_columns: SmallVec<[NodeId; 8]>,
    },
    /// Long-to-wide.
    Pivot {
        /// Pivot key column.
        key: NodeId,
        /// Value column.
        value: NodeId,
    },
    /// Time-aligned join (rows match the most-recent prior row in the right).
    AsofJoin {
        /// Predicate.
        on: NodeId,
        /// Time column on the left.
        left_time: NodeId,
        /// Time column on the right.
        right_time: NodeId,
    },
    /// Forward-fill missing buckets in a time series.
    GapFill {
        /// Time column.
        time: NodeId,
        /// Bucket interval (interpreted by the backend).
        bucket: alloc::string::String,
    },
    /// Approximate quantile sketch (t-digest).
    Tdigest {
        /// Numeric column to summarise.
        column: NodeId,
    },
    /// Approximate-anything namespace (`approx_distinct`, `approx_top_k`, …).
    Approx {
        /// Function name (e.g. `"distinct"`, `"top_k"`).
        kind: alloc::string::String,
        /// Function arguments.
        args: SmallVec<[NodeId; 4]>,
    },
    /// Limit rows to `n`.
    Limit {
        /// Row cap.
        n: u64,
    },
    /// Skip the first `n` rows.
    Offset {
        /// Number of rows to skip.
        n: u64,
    },
}

/// Inner / outer / cross join.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum JoinKind {
    /// Inner join.
    Inner,
    /// Left outer join.
    Left,
    /// Right outer join.
    Right,
    /// Full outer join.
    Full,
    /// Cross join.
    Cross,
}

/// Where rows leave the graph.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Sink {
    /// Append rows to an entity.
    Entity {
        /// Fully-qualified entity address.
        target: alloc::string::String,
    },
    /// Write rows to a file.
    File {
        /// Path.
        path: alloc::string::String,
        /// Format.
        format: alloc::string::String,
    },
    /// Write rows to an object store.
    Object {
        /// `bucket/prefix` location.
        location: alloc::string::String,
    },
    /// Return rows back through a named parameter.
    Parameter {
        /// Parameter name.
        name: alloc::string::String,
    },
}

extern crate alloc;
