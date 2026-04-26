//! Round-trip serde tests for `dol-pipeline` public top-level types.

#![cfg(feature = "serde")]

use dol_core::DataType;
use dol_pipeline::{
    ColumnSchema, Graph, Node, NodeIdx, RowSchema, Sink, Source, Transform, node::JoinKind,
};
use smallvec::smallvec;

fn round_trip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let json = serde_json::to_string(value).expect("serialize");
    serde_json::from_str(&json).expect("deserialize")
}

fn sample_schema() -> RowSchema {
    RowSchema::empty()
        .with_column("id", DataType::Uuid, false)
        .with_column("email", DataType::unbounded_string(), true)
}

#[test]
fn column_and_row_schema_round_trip() {
    let col = ColumnSchema {
        name: "id".into(),
        ty: DataType::Uuid,
        nullable: false,
    };
    assert_eq!(col, round_trip(&col));

    let rs = sample_schema();
    assert_eq!(rs, round_trip(&rs));

    let empty = RowSchema::empty();
    assert_eq!(empty, round_trip(&empty));
}

#[test]
fn node_idx_round_trip() {
    let i = NodeIdx(7);
    assert_eq!(i, round_trip(&i));
}

#[test]
fn source_round_trip() {
    let cases = [
        Source::Entity {
            target: "public.users".into(),
            schema: sample_schema(),
        },
        Source::File {
            path: "/data/users.csv".into(),
            format: "csv".into(),
            schema: sample_schema(),
        },
        Source::Object {
            location: "s3://bucket/prefix".into(),
            schema: sample_schema(),
        },
        Source::Parameter {
            name: "rows".into(),
            schema: sample_schema(),
        },
    ];
    for s in &cases {
        assert_eq!(s, &round_trip(s));
    }
}

#[test]
fn transform_round_trip() {
    let cases = [
        Transform::Filter { predicate: 1 },
        Transform::Project {
            exprs: smallvec![1, 2, 3],
        },
        Transform::Aggregate {
            keys: smallvec![1, 2],
            aggs: smallvec![3],
        },
        Transform::Join {
            kind: JoinKind::Inner,
            on: 5,
        },
        Transform::Unnest { column: 1 },
        Transform::Unpivot {
            id_columns: smallvec![1],
            value_columns: smallvec![2, 3],
        },
        Transform::Pivot { key: 1, value: 2 },
        Transform::AsofJoin {
            on: 1,
            left_time: 2,
            right_time: 3,
        },
        Transform::GapFill {
            time: 1,
            bucket: "1m".into(),
        },
        Transform::Tdigest { column: 1 },
        Transform::Approx {
            kind: "distinct".into(),
            args: smallvec![1, 2],
        },
        Transform::Limit { n: 100 },
        Transform::Offset { n: 5 },
    ];
    for t in &cases {
        assert_eq!(t, &round_trip(t));
    }
}

#[test]
fn join_kind_round_trip() {
    for k in [
        JoinKind::Inner,
        JoinKind::Left,
        JoinKind::Right,
        JoinKind::Full,
        JoinKind::Cross,
    ] {
        assert_eq!(k, round_trip(&k));
    }
}

#[test]
fn sink_round_trip() {
    let cases = [
        Sink::Entity {
            target: "public.users".into(),
        },
        Sink::File {
            path: "/out.parquet".into(),
            format: "parquet".into(),
        },
        Sink::Object {
            location: "s3://bucket/out".into(),
        },
        Sink::Parameter {
            name: "rows".into(),
        },
    ];
    for s in &cases {
        assert_eq!(s, &round_trip(s));
    }
}

#[test]
fn graph_round_trip() {
    let mut g = Graph::new();
    let src = g.add(
        Node::Source(Source::Entity {
            target: "users".into(),
            schema: sample_schema(),
        }),
        [],
    );
    let proj = g.add(
        Node::Transform(Transform::Project {
            exprs: smallvec![1, 2],
        }),
        [src],
    );
    let snk = g.add(
        Node::Sink(Sink::Entity {
            target: "users_copy".into(),
        }),
        [proj],
    );
    g.mark_output(snk);

    assert_eq!(g, round_trip(&g));
}
