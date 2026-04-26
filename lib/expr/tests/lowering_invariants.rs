//! Lowering invariants — checks the tree → arena bridge by hand-crafting
//! representative inputs and inspecting the resulting arena/interner via
//! the public API.
//!
//! These complement (not replace) the larger end-to-end tests that live
//! in `dol-ir` and downstream backends, by isolating regressions to the
//! specific lowering rule that fired.

use dol_expr::lower::lower_expr;
use dol_expr::tree::{field, int, namespace, param, string};
use dol_expr::{ExprArena, ExprNode, Interner};

#[test]
fn lowering_a_namespace_interns_its_dotted_form() {
    let e = namespace("public.users");
    let mut arena = ExprArena::default();
    let mut interner = Interner::new();

    let root = lower_expr(&e, &mut arena, &mut interner);
    let id = match arena.get(root) {
        ExprNode::Namespace(id) => *id,
        other => panic!("expected Namespace, got {other:?}"),
    };
    assert_eq!(interner.get(id), "public.users");
}

#[test]
fn lowering_a_field_uses_a_field_pool_entry() {
    let e = field("name");

    let mut arena = ExprArena::default();
    let mut interner = Interner::new();

    let root = lower_expr(&e, &mut arena, &mut interner);
    let fid = match arena.get(root) {
        ExprNode::Field(fid) => *fid,
        other => panic!("expected Field, got {other:?}"),
    };
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

    let root = lower_expr(&e, &mut arena, &mut interner);
    match arena.get(root) {
        ExprNode::BinOp { op, lhs, rhs } => {
            assert!(matches!(op, dol_expr::BinOp::Eq));
            assert!(matches!(arena.get(*lhs), ExprNode::Field(_)));
            assert!(matches!(arena.get(*rhs), ExprNode::Param));
        }
        other => panic!("expected BinOp, got {other:?}"),
    }
}

#[test]
fn lowering_is_deterministic() {
    // Lowering the same tree twice must yield the same root id and arena
    // length (modulo pool ordering, which is itself deterministic in
    // post-order traversal).
    let make = || field("user.id").eq(int(7i32));

    let mut arena_a = ExprArena::default();
    let mut interner_a = Interner::new();
    let root_a = lower_expr(&make(), &mut arena_a, &mut interner_a);

    let mut arena_b = ExprArena::default();
    let mut interner_b = Interner::new();
    let root_b = lower_expr(&make(), &mut arena_b, &mut interner_b);

    assert_eq!(root_a, root_b);
    assert_eq!(arena_a.len(), arena_b.len());
    assert_eq!(interner_a.len(), interner_b.len());
}

#[test]
fn lowering_a_string_literal_uses_the_lits_pool() {
    let e = string("hello");
    let mut arena = ExprArena::default();
    let mut interner = Interner::new();

    let root = lower_expr(&e, &mut arena, &mut interner);
    match arena.get(root) {
        ExprNode::Lit(lid) => {
            // `get_lit` panics on an out-of-bounds id; a successful call
            // proves the index is valid for the pool.
            let _ = arena.get_lit(*lid);
        }
        other => panic!("expected Lit, got {other:?}"),
    }
}
