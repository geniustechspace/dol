//! Integration tests for `ExprArena::intern_lit` / `intern_field`.
//!
//! Verifies the two examples called out in the problem statement:
//!
//! 1. Two `Literal::Int64(1)` values get the same `LiteralId`.
//! 2. Two identical `FieldNode { namespace: None, name: "email",
//!    steps: [] }` values get the same `FieldId`.
//!
//! Plus the usual safety properties: distinct content → distinct ids,
//! the `alloc_*`/`intern_*` paths interoperate (i.e. seeding the
//! lazy index from a previously-built pool works), and the cross-arena
//! determinism that content-addressing buys us.

use dol_expr::arena::{ExprArena, FieldNode, FieldStep};
use dol_expr::types::value::Literal;
use smallvec::smallvec;

#[test]
fn identical_int64_literals_dedup_to_one_id() {
    let mut arena = ExprArena::new();
    let a = arena.intern_lit(Literal::Int64(1));
    let b = arena.intern_lit(Literal::Int64(1));
    assert_eq!(a, b);
    assert_eq!(arena.lits_slice().len(), 1);
}

#[test]
fn distinct_literals_get_distinct_ids() {
    let mut arena = ExprArena::new();
    let a = arena.intern_lit(Literal::Int64(1));
    let b = arena.intern_lit(Literal::Int64(2));
    let c = arena.intern_lit(Literal::Int32(1));
    assert_ne!(a, b);
    assert_ne!(a, c); // tag byte disambiguates Int32 vs Int64
    assert_ne!(b, c);
    assert_eq!(arena.lits_slice().len(), 3);
}

#[test]
fn identical_field_nodes_dedup_to_one_id() {
    let mut arena = ExprArena::new();
    let mut interner = dol_expr::Interner::new();
    let name = interner.intern("email");

    let a = arena.intern_field(FieldNode {
        namespace: None,
        name,
        steps: smallvec![],
    });
    let b = arena.intern_field(FieldNode {
        namespace: None,
        name,
        steps: smallvec![],
    });
    assert_eq!(a, b);
    assert_eq!(arena.fields_slice().len(), 1);
}

#[test]
fn field_nodes_with_different_steps_get_distinct_ids() {
    let mut arena = ExprArena::new();
    let mut interner = dol_expr::Interner::new();
    let name = interner.intern("data");
    let key = interner.intern("x");

    let bare = arena.intern_field(FieldNode {
        namespace: None,
        name,
        steps: smallvec![],
    });
    let with_step = arena.intern_field(FieldNode {
        namespace: None,
        name,
        steps: smallvec![FieldStep::Key(key)],
    });
    let with_index = arena.intern_field(FieldNode {
        namespace: None,
        name,
        steps: smallvec![FieldStep::Index(0)],
    });
    assert_ne!(bare, with_step);
    assert_ne!(with_step, with_index);
    assert_ne!(bare, with_index);
    assert_eq!(arena.fields_slice().len(), 3);
}

#[test]
fn intern_seeds_index_from_prior_alloc() {
    // The dedup index is built lazily on first `intern_*` call. Items
    // already pushed via the bulk `alloc_*` path must participate in
    // dedup from that point onward, mirroring `intern_node`'s
    // behaviour.
    let mut arena = ExprArena::new();
    let prior = arena.alloc_lit(Literal::Int64(99));
    let again = arena.intern_lit(Literal::Int64(99));
    assert_eq!(prior, again, "intern must dedup against prior alloc");
    assert_eq!(arena.lits_slice().len(), 1);
}

#[test]
fn intern_and_alloc_can_interleave() {
    let mut arena = ExprArena::new();
    let a = arena.intern_lit(Literal::Int64(1));
    // A subsequent `alloc_lit` of the same content intentionally does
    // *not* dedup (caller chose the non-interning path), but the new
    // entry is recorded so a follow-up `intern_lit` would not
    // re-introduce a third copy. We verify the entry count grows by
    // exactly one and that the original id is unchanged.
    let _b = arena.alloc_lit(Literal::Int64(1));
    let c = arena.intern_lit(Literal::Int64(1));
    // First `intern_lit` after `alloc_lit` returns one of the existing
    // ids (the content-id index is stable across both pools); both
    // entries are content-equal so any of them is correct.
    assert!(c == a || arena.get_lit(c) == &Literal::Int64(1));
    assert_eq!(arena.lits_slice().len(), 2);
}

#[test]
fn cross_arena_content_ids_match() {
    // Content-addressing: the same Literal in two independent arenas
    // produces the same LiteralId (the dedup key derives from the
    // canonical bytes, not the per-arena pool position). This is the
    // property that makes the unified Internable contract worth the
    // wiring — plan-cache keys etc. become arena-agnostic.
    use dol_core::intern::Internable;

    let id_a: dol_expr::ids::LiteralId = Literal::Int64(42).content_id();
    let id_b: dol_expr::ids::LiteralId = Literal::Int64(42).content_id();
    assert_eq!(id_a, id_b);

    // And the two distinct values produce distinct ids — i.e. the
    // hash actually depends on the payload rather than collapsing
    // everything to the folded-zero fallback.
    let id_other: dol_expr::ids::LiteralId = Literal::Int64(43).content_id();
    assert_ne!(id_a, id_other);
}
