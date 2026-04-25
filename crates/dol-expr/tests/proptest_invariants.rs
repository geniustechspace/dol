//! Algebraic invariants for `dol-expr` exercised with `proptest`.
//!
//! These complement the static `serde_roundtrip` corpus by randomly
//! generating inputs and checking properties that must hold for *every*
//! such input. The properties target:
//!
//! 1. `Interner` determinism and idempotence.
//! 2. `Interner`'s hand-rolled serde codec (insertion-order canonical form).
//! 3. Round-trip stability of the small enum types.

#![cfg(feature = "serde")]

use dol_expr::{BinOp, Interner, JoinType, LockHint, Order, UnaryOp};
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

    /// Interning a sequence is deterministic: the same input order produces
    /// the same `(StrId, &str)` pairs in the same positions.
    #[test]
    fn intern_is_order_deterministic(strings in proptest::collection::vec("\\PC{0,16}", 0..16)) {
        let mut a = Interner::new();
        let mut b = Interner::new();
        let ids_a: Vec<_> = strings.iter().map(|s| a.intern(s)).collect();
        let ids_b: Vec<_> = strings.iter().map(|s| b.intern(s)).collect();
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

    /// The interner serialises to its canonical `Vec<String>` form and
    /// round-trips through both JSON and any byte-stable encoder. We test
    /// JSON here because it is enabled in dev-deps; postcard is exercised
    /// end-to-end at the program level in `dol-wire`.
    #[test]
    fn intern_round_trips_through_json(
        strings in proptest::collection::vec("\\PC{0,16}", 0..16)
    ) {
        let mut original = Interner::new();
        for s in &strings {
            original.intern(s);
        }
        let json = serde_json::to_string(&original).unwrap();
        let restored: Interner = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(original.len(), restored.len());
        for s in &strings {
            // Some strings may be duplicates; only check at unique indices.
            if let Some(id) = original.try_get(s) {
                prop_assert_eq!(restored.get(id), s.as_str());
            }
        }
    }

    /// JSON serialisation of an interner is deterministic for a given
    /// insertion order — two interners populated identically produce the
    /// same JSON byte-for-byte.
    #[test]
    fn intern_json_is_insertion_order_canonical(
        strings in proptest::collection::vec("\\PC{0,16}", 0..16)
    ) {
        let make = || {
            let mut i = Interner::new();
            for s in &strings {
                i.intern(s);
            }
            i
        };
        let a = serde_json::to_string(&make()).unwrap();
        let b = serde_json::to_string(&make()).unwrap();
        prop_assert_eq!(a, b);
    }
}

// ─── Small-enum round-trip ────────────────────────────────────────────────────

fn binop_strategy() -> impl Strategy<Value = BinOp> {
    prop_oneof![
        Just(BinOp::Eq),
        Just(BinOp::Ne),
        Just(BinOp::Lt),
        Just(BinOp::Le),
        Just(BinOp::Gt),
        Just(BinOp::Ge),
        Just(BinOp::And),
        Just(BinOp::Or),
        Just(BinOp::Add),
        Just(BinOp::Sub),
        Just(BinOp::Mul),
        Just(BinOp::Div),
        Just(BinOp::Rem),
        Just(BinOp::Like),
        Just(BinOp::ILike),
        Just(BinOp::Similar),
        Just(BinOp::BitAnd),
        Just(BinOp::BitOr),
        Just(BinOp::BitXor),
        Just(BinOp::Shl),
        Just(BinOp::Shr),
        Just(BinOp::Concat),
        Just(BinOp::Arrow),
        Just(BinOp::LongArrow),
    ]
}

fn unaryop_strategy() -> impl Strategy<Value = UnaryOp> {
    prop_oneof![
        Just(UnaryOp::Neg),
        Just(UnaryOp::Not),
        Just(UnaryOp::IsNull),
        Just(UnaryOp::IsNotNull),
        Just(UnaryOp::IsTrue),
        Just(UnaryOp::IsFalse),
    ]
}

fn order_strategy() -> impl Strategy<Value = Order> {
    prop_oneof![Just(Order::Asc), Just(Order::Desc)]
}

fn jointype_strategy() -> impl Strategy<Value = JoinType> {
    prop_oneof![
        Just(JoinType::Inner),
        Just(JoinType::Left),
        Just(JoinType::Right),
        Just(JoinType::Full),
        Just(JoinType::Cross),
    ]
}

fn lockhint_strategy() -> impl Strategy<Value = LockHint> {
    prop_oneof![
        Just(LockHint::ForUpdate),
        Just(LockHint::ForShare),
        Just(LockHint::SkipLocked),
        Just(LockHint::NoWait),
    ]
}

proptest! {
    #[test]
    fn enums_round_trip(
        binop in binop_strategy(),
        unaryop in unaryop_strategy(),
        order in order_strategy(),
        jt in jointype_strategy(),
        lh in lockhint_strategy(),
    ) {
        let json = serde_json::to_string(&binop).unwrap();
        prop_assert_eq!(binop, serde_json::from_str(&json).unwrap());

        let json = serde_json::to_string(&unaryop).unwrap();
        prop_assert_eq!(unaryop, serde_json::from_str(&json).unwrap());

        let json = serde_json::to_string(&order).unwrap();
        prop_assert_eq!(order, serde_json::from_str(&json).unwrap());

        let json = serde_json::to_string(&jt).unwrap();
        prop_assert_eq!(jt, serde_json::from_str(&json).unwrap());

        let json = serde_json::to_string(&lh).unwrap();
        prop_assert_eq!(lh, serde_json::from_str(&json).unwrap());
    }
}
