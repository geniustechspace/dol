#[allow(deprecated)]
use super::*;
use super::func::FuncName;

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
    assert!(matches!(bool_expr(false), Expr::Value(Literal::Bool(false))));
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
fn test_raw_expr() {
    assert!(matches!(raw_expr("1 = 1"), Expr::Raw(s) if s == "1 = 1"));
}

#[test]
fn test_case_builder_basic() {
    let expr = case()
        .when(field("x").gt(int(0i32)), string("positive"))
        .else_(string("non-positive"))
        .end();
    assert!(
        matches!(expr, Expr::Case { whens, else_expr } if whens.len() == 1 && else_expr.is_some())
    );
}

#[test]
fn test_obj() {
    let expr = obj(vec![("key", int(1i32)), ("name", string("val"))]);
    assert!(matches!(expr, Expr::ObjectLiteral(fields) if fields.len() == 2));
}

#[test]
fn test_arr() {
    let expr = arr(vec![int(1i32), int(2i32), int(3i32)]);
    assert!(matches!(expr, Expr::ArrayLiteral(elems) if elems.len() == 3));
}

// ── 2. Comparison operators ──

#[test]
fn test_eq() {
    let e = field("a").eq(int(1i32));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Eq, .. }));
}

#[test]
fn test_ne() {
    let e = field("a").ne(int(1i32));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Ne, .. }));
}

#[test]
fn test_lt() {
    let e = field("a").lt(int(1i32));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Lt, .. }));
}

#[test]
fn test_gt() {
    let e = field("a").gt(int(1i32));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Gt, .. }));
}

#[test]
fn test_le() {
    let e = field("a").le(int(1i32));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Le, .. }));
}

#[test]
fn test_ge() {
    let e = field("a").ge(int(1i32));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Ge, .. }));
}

// ── 3. Pattern matching ──

#[test]
fn test_like() {
    let e = field("name").like(string("%foo%"));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Like, .. }));
}

#[test]
fn test_ilike() {
    let e = field("name").ilike(string("%foo%"));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::ILike, .. }));
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
    let e = field("age").between(int(18i32), int(65i32));
    assert!(matches!(e, Expr::Between { negated: false, .. }));
}

#[test]
fn test_not_between() {
    let e = field("age").not_between(int(0i32), int(17i32));
    assert!(matches!(e, Expr::Between { negated: true, .. }));
}

// ── 6. Set membership ──

#[test]
fn test_in_list() {
    let e = field("status").in_list(vec![string("a"), string("b")]);
    assert!(matches!(
        e,
        Expr::InList { negated: false, list, .. } if list.len() == 2
    ));
}

#[test]
fn test_not_in_list() {
    let e = field("status").not_in_list(vec![string("x")]);
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
        Expr::Alias { alias, .. } if alias == "name"
    ));
}

#[test]
fn test_concat() {
    let e = field("first").concat(field("last"));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Concat, .. }));
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
    let e = field("a").eq(int(1i32)) & field("b").eq(int(2i32));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::And, .. }));
}

#[test]
fn test_bitor_or() {
    let e = field("a").eq(int(1i32)) | field("b").eq(int(2i32));
    assert!(matches!(e, Expr::BinaryOp { op: BinOp::Or, .. }));
}

#[test]
fn test_not() {
    let e = !field("active");
    assert!(matches!(e, Expr::UnaryOp { op: UnaryOp::Not, .. }));
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
    assert!(matches!(e, Expr::UnaryOp { op: UnaryOp::Neg, .. }));
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
    assert!(matches!(e, Expr::FieldAccess { field, .. } if field == "c"));
}

#[test]
fn test_deep_chain() {
    let e = field("a").gt(int(1i32)) & field("b").lt(int(2i32)) & field("c").eq(int(3i32));
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
    assert!(matches!(e, Expr::Func { ref name, ref args } if *name == FuncName::Count && args.len() == 1));
}

#[test]
fn test_func_count_star() {
    assert!(matches!(func::count_star(), Expr::CountStar));
}

#[test]
fn test_func_sum() {
    let e = func::sum(field("amount"));
    assert!(matches!(e, Expr::Func { ref name, ref args } if *name == FuncName::Sum && args.len() == 1));
}

#[test]
fn test_func_lower() {
    let e = func::lower(field("email"));
    assert!(matches!(e, Expr::Func { ref name, ref args } if *name == FuncName::Lower && args.len() == 1));
}

#[test]
fn test_func_now() {
    let e = func::now();
    assert!(matches!(e, Expr::Func { ref name, ref args } if *name == FuncName::Now && args.is_empty()));
}

#[test]
fn test_func_coalesce() {
    let e = func::coalesce(vec![field("a"), field("b"), string("default")]);
    assert!(matches!(e, Expr::Func { ref name, ref args } if *name == FuncName::Coalesce && args.len() == 3));
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
        .rows_between(FrameBound::UnboundedPreceding, FrameBound::UnboundedFollowing)
        .build();
    match e {
        Expr::Window { partition_by, order_by, frame: Some(f), .. } => {
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
        Expr::BinaryOp { left, op, right, negated } => {
            assert!(matches!(*left, Expr::Identifier(s) if s == "age"));
            assert_eq!(op, BinOp::Gt);
            assert!(matches!(*right, Expr::Value(Literal::Int64(18))));
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
        Expr::Between { expr, low, high, negated } => {
            assert!(matches!(*expr, Expr::Identifier(s) if s == "score"));
            assert!(matches!(*low, Expr::Value(Literal::Int64(0))));
            assert!(matches!(*high, Expr::Value(Literal::Int64(100))));
            assert!(!negated);
        }
        other => panic!("expected Between, got {other:?}"),
    }
}

#[test]
fn test_eq_with_into_expr() {
    let e = field("status").eq("active");
    match e {
        Expr::BinaryOp { right, op, negated, .. } => {
            assert_eq!(op, BinOp::Eq);
            assert!(!negated);
            assert!(matches!(*right, Expr::Identifier(s) if s == "active"));
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
