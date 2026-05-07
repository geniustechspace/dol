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
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

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

    c.bench_function("interner/json_serialize_256", |b| {
        b.iter(|| {
            let s = serde_json::to_string(&original).unwrap();
            std::hint::black_box(s)
        });
    });

    // The JSON-deserialise bench was removed in 0.2.0 along with the
    // `Deserialize` impl on `Interner`: v2 wire-in goes through
    // `dol-wire::Decode` exclusively. The corresponding decode-throughput
    // bench lives in `dol-wire`.
}

/// Lower an `AND`-chain of growing depth. This exercises both the
/// arena-allocation hot path (one BinOp + one Field per arm) and the new
/// bounded-depth check on `BuildSession::lower`. Output is a baseline for
/// later tiers (e.g. structural dedup, packed IR).
fn lower_and_chain(c: &mut Criterion) {
    use dol_expr::session::BuildSession;
    use dol_expr::tree::Expr;
    use dol_expr::tree::{field, int};

    // Pre-build a pool of `'static` column names once. Earlier iterations
    // of this bench used `Box::leak` per arm inside `build_and_chain`,
    // which leaked an unbounded amount of memory when Criterion reran
    // setup across sample groups. Leaking once here is bounded and
    // intentional — it gives every subsequent invocation a fresh
    // `&'static str` without per-iteration churn.
    fn name(idx: usize) -> &'static str {
        use std::sync::OnceLock;
        // 1024 names is more than enough for the depths used below.
        static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
        let names = NAMES.get_or_init(|| {
            (0..1024)
                .map(|i| Box::leak(format!("c{i}").into_boxed_str()) as &'static str)
                .collect()
        });
        names[idx]
    }

    fn build_and_chain(depth: usize) -> Expr<'static> {
        let mut e = field(name(0)).eq(int(0i32));
        for i in 1..depth {
            e = e & field(name(i)).eq(int(i as i32));
        }
        e
    }

    for depth in [8usize, 64, 256].iter().copied() {
        let label = format!("lower/and_chain_{depth}");
        let expr = build_and_chain(depth);
        c.bench_function(&label, |b| {
            b.iter(|| {
                let mut sess = BuildSession::new();
                let root = sess.lower(&expr).expect("lower succeeds");
                std::hint::black_box(root);
                std::hint::black_box(sess)
            });
        });
    }
}

/// Walk every node of a representative arena. Forms the baseline for any
/// future visitor / SoA / packed-IR work; the *cost per node* is what
/// changes when the layout is changed, so a per-node throughput number is
/// what matters.
fn traverse_arena(c: &mut Criterion) {
    use dol_expr::session::BuildSession;
    use dol_expr::tree::{field, int};

    // Reuse the same name pool as `lower_and_chain` to avoid per-bench
    // leaks; we duplicate the helper inline because Criterion harnesses
    // each bench function as a separate entry point.
    fn name(idx: usize) -> &'static str {
        use std::sync::OnceLock;
        static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
        let names = NAMES.get_or_init(|| {
            (0..1024)
                .map(|i| Box::leak(format!("t{i}").into_boxed_str()) as &'static str)
                .collect()
        });
        names[idx]
    }

    // Build a moderately-sized arena: a 256-arm AND-chain of equality
    // predicates. Roughly 1000 nodes after lowering (1 field + 1 lit + 1
    // eq + 1 and per arm).
    let mut sess = BuildSession::new();
    let mut e = field(name(0)).eq(int(0i32));
    for i in 1..256 {
        e = e & field(name(i)).eq(int(i as i32));
    }
    sess.lower(&e).unwrap();
    let arena = sess.arena;

    c.bench_function("arena/traverse_256_arms", |b| {
        b.iter(|| {
            // Touch every node — sufficient to force loads from each cache
            // line. Discriminant counts give the optimiser something it
            // cannot constant-fold away.
            let mut count = 0u32;
            for i in 0..arena.len() {
                let id = dol_expr::ids::NodeId::from_index(i).expect("arena index fits in NodeId");
                if arena.get(id).as_bin().is_some() {
                    count += 1;
                }
            }
            std::hint::black_box(count)
        });
    });
}

criterion_group!(
    expr_benches,
    intern_hot_strings,
    intern_unique_strings,
    intern_json_round_trip,
    lower_and_chain,
    traverse_arena,
);
criterion_main!(expr_benches);
