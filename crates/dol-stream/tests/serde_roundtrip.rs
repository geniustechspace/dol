//! Round-trip serde tests for `dol-stream` public top-level types.

#![cfg(feature = "serde")]

use dol_stream::{
    Actuator, PayloadCodec, QoS, RetentionPolicy, Sample, Sensor, StoreAndForward, TimeSeriesOp,
    TimeUnit, Trigger, Watermark, WindowSpec,
};
use dol_types::DataType;

fn round_trip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let json = serde_json::to_string(value).expect("serialize");
    serde_json::from_str(&json).expect("deserialize")
}

#[test]
fn window_spec_round_trip() {
    let cases = [
        WindowSpec::Tumbling {
            time: 1,
            size_ms: 1_000,
        },
        WindowSpec::Hopping {
            time: 1,
            size_ms: 1_000,
            hop_ms: 250,
        },
        WindowSpec::Session {
            time: 1,
            gap_ms: 500,
        },
        WindowSpec::CountBased { rows: 100 },
    ];
    for w in &cases {
        assert_eq!(w, &round_trip(w));
    }
}

#[test]
fn watermark_round_trip() {
    let w = Watermark {
        time: 7,
        max_out_of_order_ms: 500,
        allowed_lateness_ms: 1_000,
    };
    assert_eq!(w, round_trip(&w));
}

#[test]
fn trigger_round_trip() {
    for t in [
        Trigger::AfterWatermark,
        Trigger::PerRecord,
        Trigger::Periodic { period_ms: 100 },
        Trigger::OnSignal,
    ] {
        assert_eq!(t, round_trip(&t));
    }
}

#[test]
fn time_unit_round_trip() {
    for u in [
        TimeUnit::Millisecond,
        TimeUnit::Second,
        TimeUnit::Minute,
        TimeUnit::Hour,
        TimeUnit::Day,
        TimeUnit::Week,
        TimeUnit::Month,
        TimeUnit::Year,
    ] {
        assert_eq!(u, round_trip(&u));
    }
}

#[test]
fn time_series_op_round_trip() {
    let cases = [
        TimeSeriesOp::TimeBucket {
            time: 1,
            size: 5,
            unit: TimeUnit::Minute,
        },
        TimeSeriesOp::Downsample { time: 1, factor: 4 },
        TimeSeriesOp::GapFill {
            time: 1,
            size: 1,
            unit: TimeUnit::Hour,
        },
        TimeSeriesOp::Locf { column: 2 },
        TimeSeriesOp::Rate { column: 2, time: 1 },
        TimeSeriesOp::Delta { column: 2 },
    ];
    for op in &cases {
        assert_eq!(op, &round_trip(op));
    }
}

#[test]
fn iot_round_trip() {
    let sensor = Sensor {
        id: "tempA".into(),
        reading_type: DataType::Float64,
        unit: Some("°C".into()),
        sample_interval_ms: 1_000,
    };
    assert_eq!(sensor, round_trip(&sensor));

    let actuator = Actuator {
        id: "valve".into(),
        command_type: DataType::Bool,
    };
    assert_eq!(actuator, round_trip(&actuator));

    let sample = Sample {
        sensor_id: "tempA".into(),
        timestamp_ms: 1_700_000_000_000,
        value: vec![0xde, 0xad, 0xbe, 0xef],
    };
    assert_eq!(sample, round_trip(&sample));

    for q in [QoS::AtMostOnce, QoS::AtLeastOnce, QoS::ExactlyOnce] {
        assert_eq!(q, round_trip(&q));
    }

    let policy = RetentionPolicy {
        keep_for_ms: 86_400_000,
        compact_with: Some("avg".into()),
    };
    assert_eq!(policy, round_trip(&policy));

    let snf = StoreAndForward {
        buffer_bytes: 1_048_576,
        drop_oldest_on_overflow: true,
        batch_size: 64,
    };
    assert_eq!(snf, round_trip(&snf));

    for c in [
        PayloadCodec::Postcard,
        PayloadCodec::Cbor,
        PayloadCodec::Protobuf,
        PayloadCodec::Json,
    ] {
        assert_eq!(c, round_trip(&c));
    }
}
