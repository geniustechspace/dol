use super::*;

// ── 1. Constructor functions ──

#[test]
fn test_field() {
    assert!(matches!(field("email"), Expr::Identifier(s) if s == "email"));
}

#[test]
fn test_qualified() {
    assert!(matches!(
        qualified("users", "email"),
        Expr::QualifiedIdentifier { scope, name } if scope == "users" && name == "email"
    ));
}

#[test]
fn test_lit_str() {
    assert!(matches!(
        lit("hello"),
        Expr::Literal(Literal::String(s)) if s == "hello"
    ));
}

#[test]
fn test_lit_int() {
    assert!(matches!(lit(42_i64), Expr::Literal(Literal::Int(42))));
}

#[test]
fn test_lit_float() {
    match lit(2.5_f64) {
        Expr::Literal(Literal::Float(v)) => assert!((v - 2.5).abs() < f64::EPSILON),
        other => panic!("expected Literal::Float, got {other:?}"),
    }
}

#[test]
fn test_lit_bool() {
    assert!(matches!(lit(true), Expr::Literal(Literal::Bool(true))));
    assert!(matches!(lit(false), Expr::Literal(Literal::Bool(false))));
}

#[test]
fn test_param() {
    assert!(matches!(param(), Expr::Param));
}

#[test]
fn test_raw_expr() {
    assert!(matches!(raw_expr("1 = 1"), Expr::Raw(s) if s == "1 = 1"));
}

#[test]
fn test_case_builder_basic() {
    let expr = case()
        .when(field("x").gt(lit(0)), lit("positive"))
        .else_(lit("non-positive"))
        .end();
    assert!(
        matches!(expr, Expr::Case { whens, else_expr } if whens.len() == 1 && else_expr.is_some())
    );
}

#[test]
fn test_obj() {
    let expr = obj(vec![("key", lit(1)), ("name", lit("val"))]);
    assert!(matches!(expr, Expr::ObjectLiteral(fields) if fields.len() == 2));
}

#[test]
fn test_arr() {
    let expr = arr(vec![lit(1), lit(2), lit(3)]);
    assert!(matches!(expr, Expr::ArrayLiteral(elems) if elems.len() == 3));
}

// ── 2. Comparison operators ──

#[test]
fn test_eq() {
    let e = field("a").eq(lit(1));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Eq, .. }));
}

#[test]
fn test_ne() {
    let e = field("a").ne(lit(1));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Ne, .. }));
}

#[test]
fn test_lt() {
    let e = field("a").lt(lit(1));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Lt, .. }));
}

#[test]
fn test_gt() {
    let e = field("a").gt(lit(1));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Gt, .. }));
}

#[test]
fn test_le() {
    let e = field("a").le(lit(1));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Le, .. }));
}

#[test]
fn test_ge() {
    let e = field("a").ge(lit(1));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Ge, .. }));
}

// ── 3. Pattern matching ──

#[test]
fn test_like() {
    let e = field("name").like(lit("%foo%"));
    assert!(matches!(
        e,
        Expr::BinaryOp {
            op: BinOp::Like,
            ..
        }
    ));
}

#[test]
fn test_ilike() {
    let e = field("name").ilike(lit("%foo%"));
    assert!(matches!(
        e,
        Expr::BinaryOp {
            op: BinOp::ILike,
            ..
        }
    ));
}

// ── 4. Null checks ──

#[test]
fn test_is_null() {
    let e = field("x").is_null();
    assert!(matches!(e, Expr::IsNull { negated: false, .. }));
}

#[test]
fn test_is_not_null() {
    let e = field("x").is_not_null();
    assert!(matches!(e, Expr::IsNull { negated: true, .. }));
}

// ── 5. Range ──

#[test]
fn test_between() {
    let e = field("age").between(lit(18), lit(65));
    assert!(matches!(e, Expr::Between { negated: false, .. }));
}

#[test]
fn test_not_between() {
    let e = field("age").not_between(lit(0), lit(17));
    assert!(matches!(e, Expr::Between { negated: true, .. }));
}

// ── 6. Set membership ──

#[test]
fn test_in_list() {
    let e = field("status").in_list(vec![lit("a"), lit("b")]);
    assert!(matches!(
        e,
        Expr::InList { negated: false, list, .. } if list.len() == 2
    ));
}

#[test]
fn test_not_in_list() {
    let e = field("status").not_in_list(vec![lit("x")]);
    assert!(matches!(
        e,
        Expr::InList { negated: true, list, .. } if list.len() == 1
    ));
}

#[test]
fn test_in_subquery() {
    let e = field("id").in_subquery("SELECT id FROM other");
    assert!(matches!(
        e,
        Expr::InSubquery { negated: false, subquery, .. } if subquery == "SELECT id FROM other"
    ));
}

#[test]
fn test_not_in_subquery() {
    let e = field("id").not_in_subquery("SELECT id FROM banned");
    assert!(matches!(
        e,
        Expr::InSubquery { negated: true, subquery, .. } if subquery == "SELECT id FROM banned"
    ));
}

// ── 7. Type conversion ──

#[test]
fn test_cast() {
    let e = field("price").cast("INTEGER");
    assert!(matches!(
        e,
        Expr::Cast { as_type, .. } if as_type == "INTEGER"
    ));
}

// ── 8. Decoration ──

#[test]
fn test_alias() {
    let e = field("first_name").alias("name");
    assert!(matches!(
        e,
        Expr::Alias { alias, .. } if alias == "name"
    ));
}

#[test]
fn test_concat() {
    let e = field("first").concat(field("last"));
    assert!(matches!(
        e,
        Expr::BinaryOp {
            op: BinOp::Concat,
            ..
        }
    ));
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
    let o = field("x").asc_nulls_first();
    assert_eq!(o.direction, Direction::Asc);
    assert_eq!(o.nulls, Some(NullsPosition::First));
}

#[test]
fn test_asc_nulls_last() {
    let o = field("x").asc_nulls_last();
    assert_eq!(o.direction, Direction::Asc);
    assert_eq!(o.nulls, Some(NullsPosition::Last));
}

#[test]
fn test_desc_nulls_first() {
    let o = field("x").desc_nulls_first();
    assert_eq!(o.direction, Direction::Desc);
    assert_eq!(o.nulls, Some(NullsPosition::First));
}

#[test]
fn test_desc_nulls_last() {
    let o = field("x").desc_nulls_last();
    assert_eq!(o.direction, Direction::Desc);
    assert_eq!(o.nulls, Some(NullsPosition::Last));
}

// ── 10. Window ──

#[test]
fn test_over_basic() {
    let e = func::row_number().over().build();
    assert!(matches!(
        e,
        Expr::Window { partition_by, order_by, frame, .. }
        if partition_by.is_empty() && order_by.is_empty() && frame.is_none()
    ));
}

// ── 11. Operator overloads ──

#[test]
fn test_bitand_and() {
    let e = field("a").eq(lit(1)) & field("b").eq(lit(2));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::And, .. }));
}

#[test]
fn test_bitor_or() {
    let e = field("a").eq(lit(1)) | field("b").eq(lit(2));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Or, .. }));
}

#[test]
fn test_not() {
    let e = !field("active");
    assert!(matches!(
        e,
        Expr::UnaryOp {
            op: UnaryOp::Not,
            ..
        }
    ));
}

#[test]
fn test_add() {
    let e = field("a") + field("b");
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Add, .. }));
}

#[test]
fn test_sub() {
    let e = field("a") - field("b");
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Sub, .. }));
}

#[test]
fn test_mul() {
    let e = field("a") * field("b");
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Mul, .. }));
}

#[test]
fn test_div() {
    let e = field("a") / field("b");
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Div, .. }));
}

#[test]
fn test_rem() {
    let e = field("a") % field("b");
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Mod, .. }));
}

#[test]
fn test_neg() {
    let e = -field("a");
    assert!(matches!(
        e,
        Expr::UnaryOp {
            op: UnaryOp::Neg,
            ..
        }
    ));
}

// ── 12. From impls ──

#[test]
fn test_from_str_for_expr() {
    let e: Expr = "email".into();
    assert!(matches!(e, Expr::Identifier(s) if s == "email"));
}

// ── 13. Edge cases ──

#[test]
fn test_empty_string_field() {
    assert!(matches!(field(""), Expr::Identifier(s) if s.is_empty()));
}

#[test]
fn test_empty_vec_arr() {
    assert!(matches!(arr(vec![]), Expr::ArrayLiteral(v) if v.is_empty()));
}

#[test]
fn test_empty_vec_obj() {
    assert!(matches!(obj(vec![]), Expr::ObjectLiteral(v) if v.is_empty()));
}

#[test]
fn test_nested_field_access() {
    let e = field("a").access("b").access("c");
    // outermost should be FieldAccess with field="c"
    assert!(matches!(e, Expr::FieldAccess { field, .. } if field == "c"));
}

#[test]
fn test_deep_chain() {
    // Build: (a > 1) AND (b < 2) AND (c = 3)
    let e = field("a").gt(lit(1)) & field("b").lt(lit(2)) & field("c").eq(lit(3));
    // The outermost is AND
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::And, .. }));
}

#[test]
fn test_in_list_empty() {
    let e = field("x").in_list(vec![]);
    assert!(matches!(
        e,
        Expr::InList { list, negated: false, .. } if list.is_empty()
    ));
}

// ── 14. Literal tests ──

#[test]
fn test_literal_from_str() {
    assert_eq!(Literal::from("hello"), Literal::String("hello".to_string()));
}

#[test]
fn test_literal_from_string() {
    assert_eq!(
        Literal::from(String::from("world")),
        Literal::String("world".to_string())
    );
}

#[test]
fn test_literal_from_i64() {
    assert_eq!(Literal::from(99_i64), Literal::Int(99));
}

#[test]
fn test_literal_from_i32() {
    assert_eq!(Literal::from(42_i32), Literal::Int(42));
}

#[test]
fn test_literal_from_f64() {
    assert_eq!(Literal::from(2.5_f64), Literal::Float(2.5));
}

#[test]
fn test_literal_from_bool() {
    assert_eq!(Literal::from(true), Literal::Bool(true));
    assert_eq!(Literal::from(false), Literal::Bool(false));
}

// ── 15. Function constructors ──

#[test]
fn test_func_count() {
    let e = func::count(field("id"));
    assert!(matches!(
        e,
        Expr::Func { name, args } if name == "COUNT" && args.len() == 1
    ));
}

#[test]
fn test_func_count_star() {
    assert!(matches!(func::count_star(), Expr::CountStar));
}

#[test]
fn test_func_sum() {
    let e = func::sum(field("amount"));
    assert!(matches!(
        e,
        Expr::Func { name, args } if name == "SUM" && args.len() == 1
    ));
}

#[test]
fn test_func_lower() {
    let e = func::lower(field("email"));
    assert!(matches!(
        e,
        Expr::Func { name, args } if name == "LOWER" && args.len() == 1
    ));
}

#[test]
fn test_func_now() {
    let e = func::now();
    assert!(matches!(
        e,
        Expr::Func { name, args } if name == "NOW" && args.is_empty()
    ));
}

#[test]
fn test_func_coalesce() {
    let e = func::coalesce(vec![field("a"), field("b"), lit("default")]);
    assert!(matches!(
        e,
        Expr::Func { name, args } if name == "COALESCE" && args.len() == 3
    ));
}

#[test]
fn test_func_upper() {
    let e = func::upper(field("name"));
    assert!(matches!(
        e,
        Expr::Func { name, args } if name == "UPPER" && args.len() == 1
    ));
}

// ── 16. CaseBuilder ──

#[test]
fn test_case_multiple_whens() {
    let expr = case()
        .when(field("x").gt(lit(100)), lit("high"))
        .when(field("x").gt(lit(50)), lit("medium"))
        .when(field("x").gt(lit(0)), lit("low"))
        .else_(lit("zero"))
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
    let expr = case().when(field("x").eq(lit(1)), lit("one")).end();
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
fn test_window_range_between() {
    let e = func::dense_rank()
        .over()
        .range_between(FrameBound::Preceding(5), FrameBound::Following(5))
        .build();
    match e {
        Expr::Window { frame: Some(f), .. } => {
            assert_eq!(f.kind, FrameKind::Range);
            assert_eq!(f.start, FrameBound::Preceding(5));
            assert_eq!(f.end, Some(FrameBound::Following(5)));
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
fn test_field_access_base_is_identifier() {
    let e = field("profile").access("address");
    match e {
        Expr::FieldAccess { base, field: f } => {
            assert!(matches!(*base, Expr::Identifier(s) if s == "profile"));
            assert_eq!(f, "address");
        }
        other => panic!("expected FieldAccess, got {other:?}"),
    }
}

#[test]
fn test_comparison_preserves_operands() {
    let e = field("age").gt(lit(18_i64));
    match e {
        Expr::BinaryOp {
            left,
            op,
            right,
            negated,
        } => {
            assert!(matches!(*left, Expr::Identifier(s) if s == "age"));
            assert_eq!(op, BinOp::Gt);
            assert!(matches!(*right, Expr::Literal(Literal::Int(18))));
            assert!(!negated);
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
            assert_eq!(alias, "total");
        }
        other => panic!("expected Alias, got {other:?}"),
    }
}

#[test]
fn test_cast_preserves_inner() {
    let e = lit("123").cast("INT");
    match e {
        Expr::Cast { expr, as_type } => {
            assert!(matches!(*expr, Expr::Literal(Literal::String(s)) if s == "123"));
            assert_eq!(as_type, "INT");
        }
        other => panic!("expected Cast, got {other:?}"),
    }
}

#[test]
fn test_between_preserves_bounds() {
    let e = field("score").between(lit(0_i64), lit(100_i64));
    match e {
        Expr::Between {
            expr,
            low,
            high,
            negated,
        } => {
            assert!(matches!(*expr, Expr::Identifier(s) if s == "score"));
            assert!(matches!(*low, Expr::Literal(Literal::Int(0))));
            assert!(matches!(*high, Expr::Literal(Literal::Int(100))));
            assert!(!negated);
        }
        other => panic!("expected Between, got {other:?}"),
    }
}

#[test]
fn test_eq_with_into_expr() {
    // &str implements Into<Expr> via From<&str>
    let e = field("status").eq("active");
    match e {
        Expr::BinaryOp {
            right, op, negated, ..
        } => {
            assert_eq!(op, BinOp::Eq);
            assert!(!negated);
            assert!(matches!(*right, Expr::Identifier(s) if s == "active"));
        }
        other => panic!("expected BinaryOp, got {other:?}"),
    }
}

#[test]
fn test_debug_format_not_empty() {
    let e = field("x").gt(lit(1));
    let dbg = format!("{e:?}");
    assert!(!dbg.is_empty());
    assert!(dbg.contains("BinaryOp"));
}

#[test]
fn test_clone_independence() {
    let a = field("x").eq(lit(1));
    let b = a.clone();
    // Both should produce the same debug output
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
}
