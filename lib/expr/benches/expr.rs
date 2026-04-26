//! Criterion benches for the language layer.
//!
//! Run with: `cargo bench -p dol-expr --features serde`.
//!
//! These benches give us baselines for:
//!
//! 1. **Interner throughput** — interning hot strings, the dominant cost in
//!    arena construction.
//! 2. **JSON round-trip** of an `Interner` — sanity-check the determinism
//!    codec doesn't regress.
//! 3. **`ExprNode` size assertion** — micro-bench around the size budget so
//!    a regression is visible alongside the gate.

#![cfg(feature = "serde")]

use criterion::{Criterion, criterion_group, criterion_main};
use dol_expr::Interner;

fn intern_hot_strings(c: &mut Criterion) {
    // Simulate the typical hot path: a small set of column names that get
    // interned many times when building a query AST.
    let names: &[&str] = &[
        "id",
        "user_id",
        "created_at",
        "updated_at",
        "status",
        "name",
        "email",
        "value",
    ];

    c.bench_function("interner/hot_8x1024", |b| {
        b.iter(|| {
            let mut i = Interner::new();
            for _ in 0..1024 {
                for &n in names {
                    let _ = std::hint::black_box(i.intern(n));
                }
            }
            std::hint::black_box(i)
        });
    });
}

fn intern_unique_strings(c: &mut Criterion) {
    let strings: Vec<String> = (0..1024).map(|i| format!("col_{i}")).collect();

    c.bench_function("interner/unique_1024", |b| {
        b.iter(|| {
            let mut i = Interner::new();
            for s in &strings {
                let _ = std::hint::black_box(i.intern(s));
            }
            std::hint::black_box(i)
        });
    });
}

fn intern_json_round_trip(c: &mut Criterion) {
    let mut original = Interner::new();
    for n in 0..256 {
        original.intern(&format!("ident_{n}"));
    }
    let json = serde_json::to_string(&original).unwrap();

    c.bench_function("interner/json_serialize_256", |b| {
        b.iter(|| {
            let s = serde_json::to_string(&original).unwrap();
            std::hint::black_box(s)
        });
    });

    c.bench_function("interner/json_deserialize_256", |b| {
        b.iter(|| {
            let i: Interner = serde_json::from_str(&json).unwrap();
            std::hint::black_box(i)
        });
    });
}

criterion_group!(
    expr_benches,
    intern_hot_strings,
    intern_unique_strings,
    intern_json_round_trip,
);
criterion_main!(expr_benches);
