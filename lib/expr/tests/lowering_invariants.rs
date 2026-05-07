//! Lowering invariants — checks the tree → arena bridge by hand-crafting
//! representative inputs and inspecting the resulting arena/interner via
//! the public API.
//!
//! These complement (not replace) the larger end-to-end tests that live
//! in `dol-ir` and downstream backends, by isolating regressions to the
//! specific lowering rule that fired.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use dol_expr::lower::lower_expr;
use dol_expr::tree::{field, int, namespace, param, string};
use dol_expr::{ExprArena, Interner};

#[test]
fn lowering_a_namespace_interns_its_dotted_form() {
    let e = namespace("public.users");
    let mut arena = ExprArena::default();
    let mut interner = Interner::new();

    let root = lower_expr(&e, &mut arena, &mut interner).unwrap();
    let id = arena
        .get(root)
        .as_namespace()
        .expect("expected Namespace opcode");
    assert_eq!(interner.get(id), "public.users");
}

#[test]
fn lowering_a_field_uses_a_field_pool_entry() {
    let e = field("name");

    let mut arena = ExprArena::default();
    let mut interner = Interner::new();

    let root = lower_expr(&e, &mut arena, &mut interner).unwrap();
    let fid = arena.get(root).as_field().expect("expected Field opcode");
    let payload = arena.get_field(fid);
    assert_eq!(interner.get(payload.name), "name");
    assert!(payload.namespace.is_none());
    assert!(payload.steps.is_empty());
}

#[test]
fn lowering_eq_field_to_param_yields_a_binop() {
    let e = field("x").eq(param());

    let mut arena = ExprArena::default();
    let mut interner = Interner::new();

    let root = lower_expr(&e, &mut arena, &mut interner).unwrap();
    let (op, lhs, rhs) = arena.get(root).as_bin().expect("expected Bin opcode");
    assert!(matches!(op, dol_expr::BinOp::Eq));
    assert!(arena.get(lhs).as_field().is_some());
    assert!(arena.get(rhs).as_param().is_some());
}

#[test]
fn lowering_is_deterministic() {
    // Lowering the same tree twice must yield the same root id and arena
    // length (modulo pool ordering, which is itself deterministic in
    // post-order traversal).
    let make = || field("user.id").eq(int(7i32));

    let mut arena_a = ExprArena::default();
    let mut interner_a = Interner::new();
    let root_a = lower_expr(&make(), &mut arena_a, &mut interner_a).unwrap();

    let mut arena_b = ExprArena::default();
    let mut interner_b = Interner::new();
    let root_b = lower_expr(&make(), &mut arena_b, &mut interner_b).unwrap();

    assert_eq!(root_a, root_b);
    assert_eq!(arena_a.len(), arena_b.len());
    assert_eq!(interner_a.len(), interner_b.len());
}

#[test]
fn lowering_a_string_literal_uses_the_lits_pool() {
    let e = string("hello");
    let mut arena = ExprArena::default();
    let mut interner = Interner::new();

    let root = lower_expr(&e, &mut arena, &mut interner).unwrap();
    let lid = arena.get(root).as_lit().expect("expected Lit opcode");
    // `get_lit` panics on an out-of-bounds id; a successful call
    // proves the index is valid for the pool.
    let _ = arena.get_lit(lid);
}
