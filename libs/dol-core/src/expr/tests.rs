use super::*;

// ── 1. Constructor functions ──

#[test]
fn test_field() {
    let e = field("email");
    if let Expr::Ref(ref path) = e {
        assert_eq!(path.as_single(), Some("email"));
    } else {
        panic!("expected Ref, got {e:?}");
    }
}

#[test]
fn test_qualified() {
    let e = qualified("users", "email");
    if let Expr::Ref(ref path) = e {
        assert_eq!(path.iter().collect::<Vec<_>>(), vec!["users", "email"]);
    } else {
        panic!("expected Ref, got {e:?}");
    }
}

#[test]
fn test_string_literal() {
    assert!(matches!(
        string("hello"),
        Expr::Value(Literal::String(ref s)) if s == "hello"
    ));
}

#[test]
fn test_int_i32() {
    assert!(matches!(int(42i32), Expr::Value(Literal::Int32(42))));
}

#[test]
fn test_int_i64() {
    assert!(matches!(int(99i64), Expr::Value(Literal::Int64(99))));
}

#[test]
fn test_int_u8() {
    assert!(matches!(int(255u8), Expr::Value(Literal::UInt8(255))));
}

#[test]
fn test_float_f64() {
    match float(2.5f64) {
        Expr::Value(Literal::Float64(v)) => assert!((v - 2.5).abs() < f64::EPSILON),
        other => panic!("expected Float64, got {other:?}"),
    }
}

#[test]
fn test_float_f32() {
    match float(1.23f32) {
        Expr::Value(Literal::Float32(v)) => assert!((v - 1.23).abs() < f32::EPSILON),
        other => panic!("expected Float32, got {other:?}"),
    }
}

#[test]
fn test_bool() {
    assert!(matches!(bool_expr(true), Expr::Value(Literal::Bool(true))));
    assert!(matches!(
        bool_expr(false),
        Expr::Value(Literal::Bool(false))
    ));
}

#[test]
fn test_null() {
    assert!(matches!(null(), Expr::Value(Literal::Null)));
}

#[test]
fn test_param() {
    assert!(matches!(param(), Expr::Param));
}

#[test]
fn test_case_builder_basic() {
    let expr = case()
        .when(field("x").gt(int(0i32)), string("positive"))
        .else_(string("non-positive"))
        .end();
    assert!(
        matches!(expr, Expr::Case { ref whens, ref else_expr } if whens.len() == 1 && else_expr.is_some())
    );
}

#[test]
fn test_obj() {
    let expr = obj(vec![("key", int(1i32)), ("name", string("val"))]);
    assert!(matches!(expr, Expr::Object(ref fields) if fields.len() == 2));
}

#[test]
fn test_arr() {
    let expr = arr(vec![int(1i32), int(2i32), int(3i32)]);
    assert!(matches!(expr, Expr::Array(ref elems) if elems.len() == 3));
}

// ── 2. Comparison operators ──

#[test]
fn test_eq() {
    let e = field("a").eq(int(1i32));
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::EQ));
}

#[test]
fn test_ne() {
    let e = field("a").ne(int(1i32));
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::NE));
}

#[test]
fn test_lt() {
    let e = field("a").lt(int(1i32));
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::LT));
}

#[test]
fn test_gt() {
    let e = field("a").gt(int(1i32));
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::GT));
}

#[test]
fn test_le() {
    let e = field("a").le(int(1i32));
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::LE));
}

#[test]
fn test_ge() {
    let e = field("a").ge(int(1i32));
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::GE));
}

// ── 3. Pattern matching ──

#[test]
fn test_like() {
    let e = field("name").like(string("%foo%"));
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::LIKE));
}

#[test]
fn test_ilike() {
    let e = field("name").ilike(string("%foo%"));
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::ILIKE));
}

// ── 4. Null checks ──

#[test]
fn test_is_null() {
    let e = field("x").is_null();
    assert!(matches!(e, Expr::UnaryOp { op: UnaryOp::IsNull, .. }));
}

#[test]
fn test_is_not_null() {
    let e = field("x").is_null().negate();
    assert!(matches!(e, Expr::UnaryOp { op: UnaryOp::IsNotNull, .. }));
}

// ── 5. Range ──

#[test]
fn test_between() {
    let e = field("age").between(int(18i32), int(65i32));
    assert!(matches!(e, Expr::Between { .. }));
}

#[test]
fn test_not_between() {
    let e = field("age").between(int(0i32), int(17i32)).negate();
    assert!(matches!(
        e,
        Expr::UnaryOp { op: UnaryOp::Not, ref expr } if matches!(**expr, Expr::Between { .. })
    ));
}

// ── 6. Set membership ──

#[test]
fn test_in_list() {
    let e = field("status").in_list(vec![string("a"), string("b")]);
    assert!(matches!(
        e,
        Expr::InList { ref list, .. } if list.len() == 2
    ));
}

#[test]
fn test_not_in_list() {
    let e = field("status").in_list(vec![string("x")]).negate();
    assert!(matches!(
        e,
        Expr::UnaryOp { op: UnaryOp::Not, ref expr }
        if matches!(**expr, Expr::InList { ref list, .. } if list.len() == 1)
    ));
}

// ── 7. Type conversion ──

#[test]
fn test_cast() {
    let e = field("price").cast(crate::types::DataType::Int32);
    assert!(matches!(
        e,
        Expr::Cast { ref as_type, .. } if *as_type == crate::types::DataType::Int32
    ));
}

// ── 8. Decoration ──

#[test]
fn test_alias() {
    let e = field("first_name").alias("name");
    assert!(matches!(
        e,
        Expr::Alias { ref alias, .. } if alias.as_str() == "name"
    ));
}

#[test]
fn test_concat() {
    let e = field("first").concat(field("last"));
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::CONCAT));
}

// ── 9. Ordering ──

#[test]
fn test_asc() {
    let o = field("name").asc();
    assert_eq!(o.direction, Direction::Asc);
    assert_eq!(o.nulls, None);
}

#[test]
fn test_desc() {
    let o = field("name").desc();
    assert_eq!(o.direction, Direction::Desc);
    assert_eq!(o.nulls, None);
}

#[test]
fn test_asc_nulls_first() {
    let o = field("x").asc().nulls_first();
    assert_eq!(o.direction, Direction::Asc);
    assert_eq!(o.nulls, Some(NullsPosition::First));
}

#[test]
fn test_desc_nulls_last() {
    let o = field("x").desc().nulls_last();
    assert_eq!(o.direction, Direction::Desc);
    assert_eq!(o.nulls, Some(NullsPosition::Last));
}

// ── 10. Window ──

#[test]
fn test_over_basic() {
    let e = func::row_number().over().build();
    assert!(matches!(
        e,
        Expr::Window { ref partition_by, ref order_by, ref frame, .. }
        if partition_by.is_empty() && order_by.is_empty() && frame.is_none()
    ));
}

// ── 11. Operator overloads ──

#[test]
fn test_bitand_and() {
    let e = field("a").eq(int(1i32)) & field("b").eq(int(2i32));
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::AND));
}

#[test]
fn test_bitor_or() {
    let e = field("a").eq(int(1i32)) | field("b").eq(int(2i32));
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::OR));
}

#[test]
fn test_not() {
    let e = !field("active");
    assert!(matches!(
        e,
        Expr::UnaryOp { op: UnaryOp::Not, .. }
    ));
}

#[test]
fn test_add() {
    let e = field("a") + field("b");
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::ADD));
}

#[test]
fn test_sub() {
    let e = field("a") - field("b");
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::SUB));
}

#[test]
fn test_mul() {
    let e = field("a") * field("b");
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::MUL));
}

#[test]
fn test_div() {
    let e = field("a") / field("b");
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::DIV));
}

#[test]
fn test_rem() {
    let e = field("a") % field("b");
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::MOD));
}

#[test]
fn test_neg() {
    let e = -field("a");
    assert!(matches!(
        e,
        Expr::UnaryOp { op: UnaryOp::Neg, .. }
    ));
}

// ── 12. From impls ──

#[test]
fn test_from_str_for_expr() {
    let e: Expr = "email".into();
    if let Expr::Ref(ref path) = e {
        assert_eq!(path.as_single(), Some("email"));
    } else {
        panic!("expected Ref, got {e:?}");
    }
}

// ── 13. Edge cases ──

#[test]
fn test_empty_string_field() {
    let e = field("");
    assert!(matches!(e, Expr::Ref(_)));
}

#[test]
fn test_empty_vec_arr() {
    assert!(matches!(arr(vec![]), Expr::Array(ref v) if v.is_empty()));
}

#[test]
fn test_empty_vec_obj() {
    assert!(matches!(obj(vec![]), Expr::Object(ref v) if v.is_empty()));
}

#[test]
fn test_nested_field_access() {
    let e = field("a").get("b").get("c");
    if let Expr::Access { ref path, .. } = e {
        assert_eq!(path.iter().collect::<Vec<_>>(), vec!["b", "c"]);
    } else {
        panic!("expected Access, got {e:?}");
    }
}

#[test]
fn test_deep_chain() {
    let e = field("a").gt(int(1i32)) & field("b").lt(int(2i32)) & field("c").eq(int(3i32));
    assert!(matches!(e, Expr::BinaryOp { ref op, .. } if op.name() == OpDef::AND));
}

#[test]
fn test_in_list_empty() {
    let e = field("x").in_list(vec![]);
    assert!(matches!(
        e,
        Expr::InList { ref list, .. } if list.is_empty()
    ));
}

// ── 14. Typed literal tests ──

#[test]
fn test_int_variants() {
    assert!(matches!(int(1i8), Expr::Value(Literal::Int8(1))));
    assert!(matches!(int(1i16), Expr::Value(Literal::Int16(1))));
    assert!(matches!(int(1i32), Expr::Value(Literal::Int32(1))));
    assert!(matches!(int(1i64), Expr::Value(Literal::Int64(1))));
    assert!(matches!(int(1i128), Expr::Value(Literal::Int128(1))));
    assert!(matches!(int(1u8), Expr::Value(Literal::UInt8(1))));
    assert!(matches!(int(1u16), Expr::Value(Literal::UInt16(1))));
    assert!(matches!(int(1u32), Expr::Value(Literal::UInt32(1))));
    assert!(matches!(int(1u64), Expr::Value(Literal::UInt64(1))));
    assert!(matches!(int(1u128), Expr::Value(Literal::UInt128(1))));
}

#[test]
fn test_float_variants() {
    assert!(matches!(float(1.0f32), Expr::Value(Literal::Float32(_))));
    assert!(matches!(float(1.0f64), Expr::Value(Literal::Float64(_))));
}

// ── 15. Function constructors ──

#[test]
fn test_func_count() {
    let e = func::count(field("id"));
    assert!(
        matches!(e, Expr::Func { ref name, ref args } if name.name() == FuncDef::COUNT && args.len() == 1)
    );
}

#[test]
fn test_func_count_star() {
    assert!(matches!(func::count_star(), Expr::CountStar));
}

#[test]
fn test_func_sum() {
    let e = func::sum(field("amount"));
    assert!(
        matches!(e, Expr::Func { ref name, ref args } if name.name() == FuncDef::SUM && args.len() == 1)
    );
}

#[test]
fn test_func_lower() {
    let e = func::lower(field("email"));
    assert!(
        matches!(e, Expr::Func { ref name, ref args } if name.name() == FuncDef::LOWER && args.len() == 1)
    );
}

#[test]
fn test_func_now() {
    let e = func::now();
    assert!(
        matches!(e, Expr::Func { ref name, ref args } if name.name() == FuncDef::NOW && args.is_empty())
    );
}

#[test]
fn test_func_coalesce() {
    let e = func::coalesce(vec![field("a"), field("b"), string("default")]);
    assert!(
        matches!(e, Expr::Func { ref name, ref args } if name.name() == FuncDef::COALESCE && args.len() == 3)
    );
}

// ── 16. CaseBuilder ──

#[test]
fn test_case_multiple_whens() {
    let expr = case()
        .when(field("x").gt(int(100i32)), string("high"))
        .when(field("x").gt(int(50i32)), string("medium"))
        .when(field("x").gt(int(0i32)), string("low"))
        .else_(string("zero"))
        .end();
    match expr {
        Expr::Case { whens, else_expr } => {
            assert_eq!(whens.len(), 3);
            assert!(else_expr.is_some());
        }
        other => panic!("expected Case, got {other:?}"),
    }
}

#[test]
fn test_case_no_else() {
    let expr = case().when(field("x").eq(int(1i32)), string("one")).end();
    match expr {
        Expr::Case { whens, else_expr } => {
            assert_eq!(whens.len(), 1);
            assert!(else_expr.is_none());
        }
        other => panic!("expected Case, got {other:?}"),
    }
}

// ── 17. WindowBuilder ──

#[test]
fn test_window_partition_by() {
    let e = func::row_number()
        .over()
        .partition_by(vec![field("dept")])
        .build();
    match e {
        Expr::Window { partition_by, .. } => assert_eq!(partition_by.len(), 1),
        other => panic!("expected Window, got {other:?}"),
    }
}

#[test]
fn test_window_order_by() {
    let e = func::rank()
        .over()
        .order_by(vec![field("salary").desc()])
        .build();
    match e {
        Expr::Window { order_by, .. } => {
            assert_eq!(order_by.len(), 1);
            assert_eq!(order_by[0].direction, Direction::Desc);
        }
        other => panic!("expected Window, got {other:?}"),
    }
}

#[test]
fn test_window_rows_between() {
    let e = func::row_number()
        .over()
        .rows_between(FrameBound::UnboundedPreceding, FrameBound::CurrentRow)
        .build();
    match e {
        Expr::Window { frame: Some(f), .. } => {
            assert_eq!(f.kind, FrameKind::Rows);
            assert_eq!(f.start, FrameBound::UnboundedPreceding);
            assert_eq!(f.end, Some(FrameBound::CurrentRow));
        }
        other => panic!("expected Window with frame, got {other:?}"),
    }
}

#[test]
fn test_window_full_chain() {
    let e = func::row_number()
        .over()
        .partition_by(vec![field("dept"), field("team")])
        .order_by(vec![field("hire_date").asc()])
        .rows_between(
            FrameBound::UnboundedPreceding,
            FrameBound::UnboundedFollowing,
        )
        .build();
    match e {
        Expr::Window {
            partition_by,
            order_by,
            frame: Some(f),
            ..
        } => {
            assert_eq!(partition_by.len(), 2);
            assert_eq!(order_by.len(), 1);
            assert_eq!(f.kind, FrameKind::Rows);
        }
        other => panic!("expected Window with all parts, got {other:?}"),
    }
}

// ── Additional coverage ──

#[test]
fn test_comparison_preserves_operands() {
    let e = field("age").gt(int(18i64));
    match e {
        Expr::BinaryOp { left, op, right } => {
            if let Expr::Ref(ref path) = *left {
                assert_eq!(path.as_single(), Some("age"));
            } else {
                panic!("expected Ref on left, got {left:?}");
            }
            assert_eq!(op.name(), OpDef::GT);
            assert!(matches!(*right, Expr::Value(Literal::Int64(18))));
        }
        other => panic!("expected BinaryOp, got {other:?}"),
    }
}

#[test]
fn test_alias_preserves_inner() {
    let e = func::count_star().alias("total");
    match e {
        Expr::Alias { expr, alias } => {
            assert!(matches!(*expr, Expr::CountStar));
            assert_eq!(alias.as_str(), "total");
        }
        other => panic!("expected Alias, got {other:?}"),
    }
}

#[test]
fn test_cast_preserves_inner() {
    let e = string("123").cast(crate::types::DataType::Int32);
    match e {
        Expr::Cast { expr, as_type } => {
            assert!(matches!(*expr, Expr::Value(Literal::String(ref s)) if s == "123"));
            assert_eq!(as_type, crate::types::DataType::Int32);
        }
        other => panic!("expected Cast, got {other:?}"),
    }
}

#[test]
fn test_between_preserves_bounds() {
    let e = field("score").between(int(0i64), int(100i64));
    match e {
        Expr::Between { expr, low, high } => {
            if let Expr::Ref(ref path) = *expr {
                assert_eq!(path.as_single(), Some("score"));
            } else {
                panic!("expected Ref, got {expr:?}");
            }
            assert!(matches!(*low, Expr::Value(Literal::Int64(0))));
            assert!(matches!(*high, Expr::Value(Literal::Int64(100))));
        }
        other => panic!("expected Between, got {other:?}"),
    }
}

#[test]
fn test_eq_with_into_expr() {
    let e = field("status").eq("active");
    match e {
        Expr::BinaryOp { right, op, .. } => {
            assert_eq!(op.name(), OpDef::EQ);
            if let Expr::Ref(ref path) = *right {
                assert_eq!(path.as_single(), Some("active"));
            } else {
                panic!("expected Ref on right, got {right:?}");
            }
        }
        other => panic!("expected BinaryOp, got {other:?}"),
    }
}

#[test]
fn test_debug_format_not_empty() {
    let e = field("x").gt(int(1i32));
    let dbg = format!("{e:?}");
    assert!(!dbg.is_empty());
    assert!(dbg.contains("BinaryOp"));
}

#[test]
fn test_clone_independence() {
    let a = field("x").eq(int(1i32));
    let b = a.clone();
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
}

// ── Typed constructors ──

#[test]
fn test_typed_constructors() {
    assert!(matches!(string("hello"), Expr::Value(Literal::String(ref s)) if s == "hello"));
    assert!(matches!(int(42i64), Expr::Value(Literal::Int64(42))));
    assert!(matches!(bool_expr(true), Expr::Value(Literal::Bool(true))));
}

// ── PathExpr and get() ──

#[test]
fn test_get_extends_access_path() {
    let e = field("profile").get("address").get("city");
    if let Expr::Access { base, path } = &e {
        // base should still be a Ref to "profile"
        assert!(matches!(**base, Expr::Ref(_)));
        // path has both "address" and "city"
        assert_eq!(path.iter().collect::<Vec<_>>(), vec!["address", "city"]);
    } else {
        panic!("expected Access, got {e:?}");
    }
}

#[test]
fn test_negate_double_not_eliminated() {
    let e = field("active");
    let negated = e.clone().negate();
    let double_negated = negated.negate();
    // NOT NOT e → e
    assert!(matches!(double_negated, Expr::Ref(_)));
}

#[test]
fn test_field_dyn() {
    let name = String::from("dynamic");
    let e = field_dyn(&name);
    if let Expr::Ref(ref path) = e {
        assert_eq!(path.as_single(), Some("dynamic"));
    } else {
        panic!("expected Ref, got {e:?}");
    }
}

#[test]
#[ignore = "size probe — run manually"]
fn size_probe() {
    use std::mem::size_of;
    use crate::expr::{CompactName, FuncDef, Expr, Literal, WindowFrame};
    use crate::expr::op_meta::OpDef;
    use crate::expr::ops::UnaryOp;
    use crate::types::DataType;
    println!("CompactName: {}", size_of::<CompactName>());
    println!("FuncDef:     {}", size_of::<FuncDef>());
    println!("OpDef:       {}", size_of::<OpDef>());
    println!("UnaryOp:     {}", size_of::<UnaryOp>());
    println!("Literal:     {}", size_of::<Literal<'static>>());
    println!("DataType:    {}", size_of::<DataType>());
    println!("WindowFrame: {}", size_of::<WindowFrame>());
    println!("Expr:        {}", size_of::<Expr<'static>>());
}

// ── Arena (Phase 2) ──────────────────────────────────────────────────────────

#[test]
fn expr_node_size_within_budget() {
    use std::mem::size_of;
    use crate::expr::node::ExprNode;
    let sz = size_of::<ExprNode>();
    assert!(
        sz <= 48,
        "ExprNode size is {sz} bytes, must be ≤ 48; box a large variant",
    );
}

#[test]
fn interner_deduplicates_strings() {
    use crate::expr::interner::Interner;
    let mut i = Interner::new();
    let a = i.intern("email");
    let b = i.intern("email");
    let c = i.intern("name");
    assert_eq!(a, b, "same string should yield same StrId");
    assert_ne!(a, c, "different strings should yield different StrIds");
    assert_eq!(i.get(a), "email");
    assert_eq!(i.get(c), "name");
    assert_eq!(i.len(), 2);
}

#[test]
fn arena_lower_ref() {
    use crate::expr::arena::ExprArena;
    use crate::expr::interner::Interner;
    use crate::expr::node::ExprNode;

    let mut arena    = ExprArena::new();
    let mut interner = Interner::new();

    let expr = field("age");
    let id   = arena.lower(&expr, &mut interner);

    assert_eq!(arena.len(), 1);
    match arena.get(id) {
        ExprNode::Ref(path) => {
            assert_eq!(path.len(), 1);
            assert_eq!(interner.get(path[0]), "age");
        }
        other => panic!("expected Ref, got {other:?}"),
    }
}

#[test]
fn arena_lower_binary_op() {
    use crate::expr::arena::ExprArena;
    use crate::expr::interner::Interner;
    use crate::expr::node::ExprNode;

    let mut arena    = ExprArena::new();
    let mut interner = Interner::new();

    let expr = field("age").gt(int(18i32));
    let root = arena.lower(&expr, &mut interner);

    // Tree: BinaryOp( Ref("age"), GT, Value(Int32(18)) )
    // Post-order: Ref @ 0, Value @ 1, BinaryOp @ 2
    assert_eq!(arena.len(), 3);
    match arena.get(root) {
        ExprNode::BinaryOp { left, op, right } => {
            assert!(matches!(arena.get(*left),  ExprNode::Ref(_)));
            assert!(matches!(arena.get(*right), ExprNode::Value(_)));
            assert_eq!(arena.op(*op).name(), "GT");
        }
        other => panic!("expected BinaryOp, got {other:?}"),
    }
}

#[test]
fn arena_lower_qualified_ref() {
    use crate::expr::arena::ExprArena;
    use crate::expr::interner::Interner;
    use crate::expr::node::ExprNode;

    let mut arena    = ExprArena::new();
    let mut interner = Interner::new();

    let expr = qualified("users", "email");
    let id   = arena.lower(&expr, &mut interner);

    match arena.get(id) {
        ExprNode::Ref(path) => {
            assert_eq!(path.len(), 2);
            assert_eq!(interner.get(path[0]), "users");
            assert_eq!(interner.get(path[1]), "email");
        }
        other => panic!("expected Ref, got {other:?}"),
    }
}

#[test]
fn arena_string_dedup_across_nodes() {
    use crate::expr::arena::ExprArena;
    use crate::expr::interner::Interner;
    use crate::expr::node::ExprNode;

    let mut arena    = ExprArena::new();
    let mut interner = Interner::new();

    // Two refs to the same field name should share the same StrId.
    let a = arena.lower(&field("id"), &mut interner);
    let b = arena.lower(&field("id"), &mut interner);

    let id_a = match arena.get(a) { ExprNode::Ref(p) => p[0], _ => panic!() };
    let id_b = match arena.get(b) { ExprNode::Ref(p) => p[0], _ => panic!() };
    assert_eq!(id_a, id_b, "same field name must share StrId");
    assert_eq!(interner.len(), 1, "only one unique string");
}

// ── Passes (Phase 3) ─────────────────────────────────────────────────────────

mod pass_tests {
    use crate::expr::arena::ExprArena;
    use crate::expr::interner::Interner;
    use crate::expr::pass::{
        AllowList, ExprContext, PassCaps, PassChain, PassError, Schema,
        run_passes,
    };
    use crate::expr::pass::arity::ArityPass;
    use crate::expr::pass::node_count::NodeCountPass;
    use crate::expr::pass::scope::ScopePass;
    use crate::expr::pass::security::SecurityPass;
    use crate::expr::pass::semantic::SemanticPass;
    use crate::expr::{field, int, string};
    use crate::expr::func::{count, row_number};

    fn lower(expr: &crate::expr::Expr<'_>) -> (ExprArena, Interner) {
        let mut arena = ExprArena::new();
        let mut interner = Interner::new();
        arena.lower(expr, &mut interner);
        (arena, interner)
    }

    // ── NodeCountPass ────────────────────────────────────────────────────────

    #[test]
    fn node_count_at_limit_passes() {
        let (arena, _) = lower(&field("x"));
        let caps = PassCaps { max_nodes: 1, ..Default::default() };
        assert!(NodeCountPass.check(&arena, &caps).is_empty());
    }

    #[test]
    fn node_count_over_limit_fails() {
        // field("a").gt(int(1)) → 3 nodes
        let (arena, _) = lower(&field("a").gt(int(1i32)));
        let caps = PassCaps { max_nodes: 2, ..Default::default() };
        let errs = NodeCountPass.check(&arena, &caps);
        assert_eq!(errs.len(), 1);
        assert!(matches!(&errs[0], PassError::TooManyNodes { count: 3, limit: 2 }));
    }

    // ── ArityPass ────────────────────────────────────────────────────────────

    #[test]
    fn arity_pass_correct_args_passes() {
        // count(field) — COUNT takes exactly 1 arg
        let (arena, _) = lower(&count(field("value")));
        assert!(ArityPass.check(&arena).is_empty());
    }

    #[test]
    fn arity_pass_bad_args_fails() {
        use crate::expr::{Expr, FuncDef, FuncKind, Arity};
        // Manually build COUNT with 2 args (wrong arity).
        let expr = Expr::Func {
            name: FuncDef::new_static("COUNT", Arity::Exact(1), FuncKind::Aggregate),
            args: vec![field("a"), field("b")],
        };
        let (arena, _) = lower(&expr);
        let errs = ArityPass.check(&arena);
        assert_eq!(errs.len(), 1);
        assert!(matches!(&errs[0], PassError::BadArity { func_name, actual: 2, .. }
            if func_name == "COUNT"));
    }

    // ── ScopePass ────────────────────────────────────────────────────────────

    #[test]
    fn scope_pass_no_schema_is_noop() {
        let (arena, interner) = lower(&field("anything"));
        let errs = ScopePass.check(&arena, &interner, None);
        assert!(errs.is_empty());
    }

    #[test]
    fn scope_pass_known_field_passes() {
        let schema = Schema::from_fields(["age", "email"]);
        let (arena, interner) = lower(&field("age"));
        let errs = ScopePass.check(&arena, &interner, Some(&schema));
        assert!(errs.is_empty());
    }

    #[test]
    fn scope_pass_unknown_field_fails() {
        let schema = Schema::from_fields(["age"]);
        let (arena, interner) = lower(&field("password_hash"));
        let errs = ScopePass.check(&arena, &interner, Some(&schema));
        assert_eq!(errs.len(), 1);
        assert!(matches!(&errs[0], PassError::UnknownField { path }
            if path == "password_hash"));
    }

    // ── SemanticPass ─────────────────────────────────────────────────────────

    #[test]
    fn aggregate_in_select_passes() {
        let (arena, _) = lower(&count(field("id")));
        assert!(SemanticPass.check(&arena, ExprContext::Select).is_empty());
    }

    #[test]
    fn aggregate_in_where_fails() {
        let (arena, _) = lower(&count(field("id")));
        let errs = SemanticPass.check(&arena, ExprContext::Where);
        assert_eq!(errs.len(), 1);
        assert!(matches!(&errs[0], PassError::AggregateInWrongContext {
            func_name, context: ExprContext::Where
        } if func_name == "COUNT"));
    }

    #[test]
    fn window_func_in_where_fails() {
        let (arena, _) = lower(&row_number());
        let errs = SemanticPass.check(&arena, ExprContext::Where);
        assert_eq!(errs.len(), 1);
        assert!(matches!(&errs[0], PassError::WindowInWrongContext {
            context: ExprContext::Where, ..
        }));
    }

    #[test]
    fn window_func_in_having_fails() {
        let (arena, _) = lower(&row_number());
        let errs = SemanticPass.check(&arena, ExprContext::Having);
        assert!(!errs.is_empty());
    }

    // ── SecurityPass ─────────────────────────────────────────────────────────

    #[test]
    fn security_no_allowlist_permits_all() {
        let (arena, interner) = lower(&count(field("id")));
        let caps = PassCaps::default();
        assert!(SecurityPass.check(&arena, &interner, &caps, None).is_empty());
    }

    #[test]
    fn security_allowlist_permits_listed_func() {
        let allow = AllowList::from_funcs(["COUNT"]);
        let (arena, interner) = lower(&count(field("id")));
        let caps = PassCaps::default();
        assert!(SecurityPass.check(&arena, &interner, &caps, Some(&allow)).is_empty());
    }

    #[test]
    fn security_allowlist_rejects_unlisted_func() {
        let allow = AllowList::from_funcs(["LOWER"]); // COUNT not in list
        let (arena, interner) = lower(&count(field("id")));
        let caps = PassCaps::default();
        let errs = SecurityPass.check(&arena, &interner, &caps, Some(&allow));
        assert_eq!(errs.len(), 1);
        assert!(matches!(&errs[0], PassError::DisallowedFunction { func_name }
            if func_name == "COUNT"));
    }

    #[test]
    fn security_string_within_limit_passes() {
        let (arena, interner) = lower(&string("hello"));
        let caps = PassCaps { max_string_len: Some(10), ..Default::default() };
        assert!(SecurityPass.check(&arena, &interner, &caps, None).is_empty());
    }

    #[test]
    fn security_string_over_limit_fails() {
        let (arena, interner) = lower(&string("exceeds_limit"));
        let caps = PassCaps { max_string_len: Some(5), ..Default::default() };
        let errs = SecurityPass.check(&arena, &interner, &caps, None);
        assert_eq!(errs.len(), 1);
        assert!(matches!(&errs[0], PassError::StringTooLong { length: 13, limit: 5 }));
    }

    // ── run_passes (chain) ───────────────────────────────────────────────────

    #[test]
    fn run_passes_clean_expr_returns_empty() {
        let schema = Schema::from_fields(["age"]);
        let (arena, interner) = lower(&field("age").gt(int(18i32)));
        let chain = PassChain {
            caps:    PassCaps::default(),
            schema:  Some(&schema),
            context: ExprContext::Where,
            allow:   None,
        };
        assert!(run_passes(&arena, &interner, &chain).is_empty());
    }

    #[test]
    fn run_passes_stops_at_first_failing_pass() {
        // node limit of 1 but we have 3 nodes — NodeCountPass fires first.
        let (arena, interner) = lower(&field("a").gt(int(1i32)));
        let chain = PassChain {
            caps:    PassCaps { max_nodes: 1, ..Default::default() },
            schema:  None,
            context: ExprContext::Where,
            allow:   None,
        };
        let errs = run_passes(&arena, &interner, &chain);
        assert_eq!(errs.len(), 1);
        assert!(matches!(errs[0], PassError::TooManyNodes { .. }));
    }
}
