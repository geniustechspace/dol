//! Algebraic invariants for `dol-expr` exercised with `proptest`.
//!
//! These complement the per-crate test corpus by randomly generating
//! inputs and checking properties that must hold for *every* such
//! input. The properties target `Interner` determinism and
//! idempotence; v2 wire round-trip is exercised end-to-end at the
//! program level in `dol-wire`'s `Encode` / `Decode` test suite.

#![cfg(feature = "serde")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use dol_expr::Interner;
use proptest::prelude::*;

// ─── Interner properties ──────────────────────────────────────────────────────

proptest! {
    /// Interning the same string twice yields the same id (idempotence) and
    /// `try_get` agrees with `intern`.
    #[test]
    fn intern_is_idempotent(s in "\\PC{0,32}") {
        let mut i = Interner::new();
        let a = i.intern(&s);
        let b = i.intern(&s);
        prop_assert_eq!(a, b);
        prop_assert_eq!(i.try_get(&s), Some(a));
        prop_assert_eq!(i.get(a), s.as_str());
    }

    /// Interning a sequence is deterministic and content-addressed: the
    /// id assigned to each input is independent of insertion order, so
    /// two interners populated identically in any order agree on every
    /// `(StrId, &str)` pair.
    #[test]
    fn intern_is_order_deterministic(strings in proptest::collection::vec("\\PC{0,16}", 0..16)) {
        let mut a = Interner::new();
        let mut b = Interner::new();
        let ids_a: Vec<_> = strings.iter().map(|s| a.intern(s)).collect();
        // Reverse the insertion order on `b` to exercise the
        // content-addressed property.
        let mut reversed: Vec<_> = strings.clone();
        reversed.reverse();
        for s in &reversed {
            b.intern(s);
        }
        let ids_b: Vec<_> = strings.iter().map(|s| b.try_get(s).expect("just interned")).collect();
        prop_assert_eq!(ids_a, ids_b);
        prop_assert_eq!(a.len(), b.len());
        // Each interned id must round-trip through `get`. The number of
        // interned ids equals the number of *unique* strings, so we walk
        // over the unique pool length, not the input length.
        for s in &strings {
            let id = a.try_get(s).expect("just interned");
            prop_assert_eq!(a.get(id), s.as_str());
            prop_assert_eq!(b.get(id), s.as_str());
        }
    }

    /// JSON serialisation of an interner is **content-canonical**:
    /// inserting the same set of strings in different orders produces
    /// the same JSON byte-for-byte.
    #[test]
    fn intern_json_is_content_canonical(
        strings in proptest::collection::vec("\\PC{0,16}", 0..16)
    ) {
        let make = |order: &[String]| {
            let mut i = Interner::new();
            for s in order {
                i.intern(s);
            }
            i
        };
        let mut reversed: Vec<_> = strings.clone();
        reversed.reverse();
        let a = serde_json::to_string(&make(&strings)).unwrap();
        let b = serde_json::to_string(&make(&reversed)).unwrap();
        prop_assert_eq!(a, b);
    }
}

// Round-trip tests for `Interner`, `BinOp`, `UnaryOp`, `Order`,
// `JoinType`, `LockHint` were removed in 0.2.0 along with `Deserialize`.
// Wire-level round-trip is exercised end-to-end at the program level
// in `dol-wire`'s `Encode` / `Decode` test suite.
