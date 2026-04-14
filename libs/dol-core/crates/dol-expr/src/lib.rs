//! # dol-core — DOL Expression Engine
//!
//! Composable, backend-agnostic expression AST for DOL.
//!
//! Expressions are the universal building blocks used across all DOL domains
//! (queries, mutations, definitions, etc.). They are backend-agnostic and
//! can be rendered to SQL, document queries, or any other target.
//!
//! # Constructors
//!
//! ```rust
//! use dol_expr::{field, col, lit, param, raw_expr, case};
//!
//! // field reference (DOL primary)
//! let expr = field("email");
//!
//! // backward-compat alias
//! let expr = col("email");
//!
//! // literal value
//! let expr = lit("active");
//!
//! // comparison chain
//! let expr = field("age").gt(lit(18)) & field("status").eq(lit("active"));
//! ```

#![deny(unsafe_code)]

pub mod func;
mod literal;
mod ops;
pub mod order;
pub mod window;

pub use literal::Literal;
pub use ops::{BinOp, UnaryOp};
pub use order::{Direction, NullsPosition, OrderByExpr};
pub use window::{CaseBuilder, FrameBound, FrameKind, WindowBuilder, WindowFrame};

use std::ops as std_ops;

/// A composable expression node — the core AST type for DOL.
///
/// Every expression is backend-agnostic. Backends (SQL, document, KV) interpret
/// and render expressions according to their own semantics.
///
/// # Examples
///
/// ```rust
/// use dol_expr::{field, lit};
///
/// // Simple comparison: age > 18
/// let expr = field("age").gt(lit(18));
///
/// // Compound: age > 18 AND status = 'active'
/// let expr = field("age").gt(lit(18)) & field("status").eq(lit("active"));
///
/// // Field access for nested data: profile.address.city
/// let expr = field("profile").access("address").access("city");
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Expr {
    // ── Identifiers ──
    /// A field/column reference: `field_name`.
    Identifier(String),
    /// A qualified field reference: `scope.name` (e.g., `users.email`).
    QualifiedIdentifier { scope: String, name: String },
    /// Nested field access: `base.field` (e.g., `profile.address.city`).
    FieldAccess { base: Box<Expr>, field: String },

    // ── Values ──
    /// A positional bind parameter (`$1`, `?`, `@p1`, `:1`).
    Param,
    /// A literal value.
    Literal(Literal),

    // ── Operations ──
    /// A binary operation: `left op right`.
    BinaryOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    /// A unary operation: `op expr`.
    UnaryOp { op: UnaryOp, expr: Box<Expr> },

    // ── Function call ──
    /// A function call: `name(args...)`.
    Func { name: String, args: Vec<Expr> },

    // ── Type conversion ──
    /// A type cast: `CAST(expr AS type)`.
    Cast { expr: Box<Expr>, as_type: String },

    // ── Conditional ──
    /// A CASE expression: `CASE WHEN ... THEN ... ELSE ... END`.
    Case {
        whens: Vec<(Expr, Expr)>,
        else_expr: Option<Box<Expr>>,
    },

    // ── Subquery ──
    /// A subquery: `(SELECT ...)`.
    Subquery(String),

    // ── Set membership ──
    /// `expr [NOT] IN (list)`.
    InList {
        expr: Box<Expr>,
        list: Vec<Expr>,
        negated: bool,
    },
    /// `expr [NOT] IN (subquery)`.
    InSubquery {
        expr: Box<Expr>,
        subquery: String,
        negated: bool,
    },

    // ── Range ──
    /// `expr [NOT] BETWEEN low AND high`.
    Between {
        expr: Box<Expr>,
        low: Box<Expr>,
        high: Box<Expr>,
        negated: bool,
    },

    // ── Existence ──
    /// `[NOT] EXISTS (subquery)`.
    Exists { subquery: String, negated: bool },

    // ── Null check ──
    /// `expr IS [NOT] NULL`.
    IsNull { expr: Box<Expr>, negated: bool },

    // ── Composites ──
    /// An object literal: `{ key: value, key2: value2 }`.
    ObjectLiteral(Vec<(String, Expr)>),
    /// An array literal: `[1, 2, 3]`.
    ArrayLiteral(Vec<Expr>),

    // ── Escape hatch ──
    /// Raw expression string (escape hatch).
    Raw(String),

    // ── Decoration ──
    /// `expr AS alias`.
    Alias { expr: Box<Expr>, alias: String },
    /// `*` (all fields/columns).
    Star,
    /// `COUNT(*)`.
    CountStar,
    /// A window function: `func OVER (PARTITION BY ... ORDER BY ... frame)`.
    Window {
        func: Box<Expr>,
        partition_by: Vec<Expr>,
        order_by: Vec<OrderByExpr>,
        frame: Option<WindowFrame>,
    },
}

// ---------------------------------------------------------------------------
// Constructor functions
// ---------------------------------------------------------------------------

/// Create a field/identifier reference expression (DOL primary constructor).
pub fn field(name: &str) -> Expr {
    Expr::Identifier(name.to_string())
}

/// Create a field reference — backward-compat alias for [`field()`].
pub fn col(name: &str) -> Expr {
    Expr::Identifier(name.to_string())
}

/// Create a qualified field reference: `scope.name`.
pub fn qualified(scope: &str, name: &str) -> Expr {
    Expr::QualifiedIdentifier {
        scope: scope.to_string(),
        name: name.to_string(),
    }
}

/// Create a qualified field reference — backward-compat alias for [`qualified()`].
pub fn qualified_col(table: &str, column: &str) -> Expr {
    qualified(table, column)
}

/// Create a literal expression from any value that implements `Into<Literal>`.
pub fn lit<T: Into<Literal>>(val: T) -> Expr {
    Expr::Literal(val.into())
}

/// Create a bind parameter expression.
pub fn param() -> Expr {
    Expr::Param
}

/// Create a raw expression string (escape hatch).
pub fn raw_expr(sql: &str) -> Expr {
    Expr::Raw(sql.to_string())
}

/// Start building a CASE expression.
pub fn case() -> CaseBuilder {
    CaseBuilder::new()
}

/// Create an object literal expression: `{ key: value, ... }`.
pub fn obj(fields: Vec<(&str, Expr)>) -> Expr {
    Expr::ObjectLiteral(
        fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

/// Create an array literal expression: `[elem1, elem2, ...]`.
pub fn arr(elements: Vec<Expr>) -> Expr {
    Expr::ArrayLiteral(elements)
}

/// Convert `&str` to `Expr::Identifier` for ergonomic builder use.
impl From<&str> for Expr {
    fn from(s: &str) -> Self {
        Expr::Identifier(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// Expr method chains
// ---------------------------------------------------------------------------

impl Expr {
    // ── Nested field access ──

    /// Nested field access: `self.field_name`.
    ///
    /// ```rust
    /// use dol_expr::field;
    /// let expr = field("profile").access("address").access("city");
    /// ```
    pub fn access(self, name: &str) -> Expr {
        Expr::FieldAccess {
            base: Box::new(self),
            field: name.to_string(),
        }
    }

    // ── Comparison operators ──

    /// `self == rhs`
    pub fn eq(self, rhs: impl Into<Expr>) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Eq,
            right: Box::new(rhs.into()),
        }
    }

    /// `self != rhs`
    pub fn ne(self, rhs: impl Into<Expr>) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Ne,
            right: Box::new(rhs.into()),
        }
    }

    /// `self < rhs`
    pub fn lt(self, rhs: impl Into<Expr>) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Lt,
            right: Box::new(rhs.into()),
        }
    }

    /// `self > rhs`
    pub fn gt(self, rhs: impl Into<Expr>) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Gt,
            right: Box::new(rhs.into()),
        }
    }

    /// `self <= rhs`
    pub fn le(self, rhs: impl Into<Expr>) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Le,
            right: Box::new(rhs.into()),
        }
    }

    /// `self >= rhs`
    pub fn ge(self, rhs: impl Into<Expr>) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Ge,
            right: Box::new(rhs.into()),
        }
    }

    // ── Pattern matching ──

    /// `self LIKE rhs`
    pub fn like(self, rhs: impl Into<Expr>) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Like,
            right: Box::new(rhs.into()),
        }
    }

    /// `self ILIKE rhs` (dialect-aware: falls back to `LOWER(self) LIKE LOWER(rhs)`).
    pub fn ilike(self, rhs: impl Into<Expr>) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::ILike,
            right: Box::new(rhs.into()),
        }
    }

    // ── Null checks ──

    /// `self IS NULL` / `self is null`
    pub fn is_null(self) -> Expr {
        Expr::IsNull {
            expr: Box::new(self),
            negated: false,
        }
    }

    /// `self IS NOT NULL` / `self is not null`
    pub fn is_not_null(self) -> Expr {
        Expr::IsNull {
            expr: Box::new(self),
            negated: true,
        }
    }

    // ── Range ──

    /// `self BETWEEN low AND high`
    pub fn between(self, low: impl Into<Expr>, high: impl Into<Expr>) -> Expr {
        Expr::Between {
            expr: Box::new(self),
            low: Box::new(low.into()),
            high: Box::new(high.into()),
            negated: false,
        }
    }

    /// `self NOT BETWEEN low AND high`
    pub fn not_between(self, low: impl Into<Expr>, high: impl Into<Expr>) -> Expr {
        Expr::Between {
            expr: Box::new(self),
            low: Box::new(low.into()),
            high: Box::new(high.into()),
            negated: true,
        }
    }

    // ── Set membership ──

    /// `self IN (list...)`
    pub fn in_list(self, list: Vec<Expr>) -> Expr {
        Expr::InList {
            expr: Box::new(self),
            list,
            negated: false,
        }
    }

    /// `self NOT IN (list...)`
    pub fn not_in_list(self, list: Vec<Expr>) -> Expr {
        Expr::InList {
            expr: Box::new(self),
            list,
            negated: true,
        }
    }

    /// `self IN (subquery)`
    pub fn in_subquery(self, subquery: &str) -> Expr {
        Expr::InSubquery {
            expr: Box::new(self),
            subquery: subquery.to_string(),
            negated: false,
        }
    }

    /// `self NOT IN (subquery)`
    pub fn not_in_subquery(self, subquery: &str) -> Expr {
        Expr::InSubquery {
            expr: Box::new(self),
            subquery: subquery.to_string(),
            negated: true,
        }
    }

    // ── Type conversion ──

    /// `CAST(self AS type)`
    pub fn cast(self, as_type: &str) -> Expr {
        Expr::Cast {
            expr: Box::new(self),
            as_type: as_type.to_string(),
        }
    }

    // ── Decoration ──

    /// `self AS alias`
    pub fn alias(self, name: &str) -> Expr {
        Expr::Alias {
            expr: Box::new(self),
            alias: name.to_string(),
        }
    }

    /// String concatenation (dialect-aware: `||` vs `CONCAT()` vs `+`).
    pub fn concat(self, rhs: impl Into<Expr>) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Concat,
            right: Box::new(rhs.into()),
        }
    }

    // ── Ordering ──

    /// `self ASC`
    pub fn asc(self) -> OrderByExpr {
        OrderByExpr {
            expr: self,
            direction: Direction::Asc,
            nulls: None,
        }
    }

    /// `self DESC`
    pub fn desc(self) -> OrderByExpr {
        OrderByExpr {
            expr: self,
            direction: Direction::Desc,
            nulls: None,
        }
    }

    /// `self ASC NULLS FIRST`
    pub fn asc_nulls_first(self) -> OrderByExpr {
        OrderByExpr {
            expr: self,
            direction: Direction::Asc,
            nulls: Some(NullsPosition::First),
        }
    }

    /// `self ASC NULLS LAST`
    pub fn asc_nulls_last(self) -> OrderByExpr {
        OrderByExpr {
            expr: self,
            direction: Direction::Asc,
            nulls: Some(NullsPosition::Last),
        }
    }

    /// `self DESC NULLS FIRST`
    pub fn desc_nulls_first(self) -> OrderByExpr {
        OrderByExpr {
            expr: self,
            direction: Direction::Desc,
            nulls: Some(NullsPosition::First),
        }
    }

    /// `self DESC NULLS LAST`
    pub fn desc_nulls_last(self) -> OrderByExpr {
        OrderByExpr {
            expr: self,
            direction: Direction::Desc,
            nulls: Some(NullsPosition::Last),
        }
    }

    // ── Window ──

    /// Start building a window function: `self OVER (...)`.
    pub fn over(self) -> WindowBuilder {
        WindowBuilder::new(self)
    }
}

// ---------------------------------------------------------------------------
// Operator overloads
// ---------------------------------------------------------------------------

/// `expr_a & expr_b` → `expr_a AND expr_b`
impl std_ops::BitAnd for Expr {
    type Output = Expr;
    fn bitand(self, rhs: Expr) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::And,
            right: Box::new(rhs),
        }
    }
}

/// `expr_a | expr_b` → `expr_a OR expr_b`
impl std_ops::BitOr for Expr {
    type Output = Expr;
    fn bitor(self, rhs: Expr) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Or,
            right: Box::new(rhs),
        }
    }
}

/// `!expr` → `NOT expr`
impl std_ops::Not for Expr {
    type Output = Expr;
    fn not(self) -> Expr {
        Expr::UnaryOp {
            op: UnaryOp::Not,
            expr: Box::new(self),
        }
    }
}

/// `expr_a + expr_b`
impl std_ops::Add for Expr {
    type Output = Expr;
    fn add(self, rhs: Expr) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Add,
            right: Box::new(rhs),
        }
    }
}

/// `expr_a - expr_b`
impl std_ops::Sub for Expr {
    type Output = Expr;
    fn sub(self, rhs: Expr) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Sub,
            right: Box::new(rhs),
        }
    }
}

/// `expr_a * expr_b`
impl std_ops::Mul for Expr {
    type Output = Expr;
    fn mul(self, rhs: Expr) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Mul,
            right: Box::new(rhs),
        }
    }
}

/// `expr_a / expr_b`
impl std_ops::Div for Expr {
    type Output = Expr;
    fn div(self, rhs: Expr) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Div,
            right: Box::new(rhs),
        }
    }
}

/// `expr_a % expr_b`
impl std_ops::Rem for Expr {
    type Output = Expr;
    fn rem(self, rhs: Expr) -> Expr {
        Expr::BinaryOp {
            left: Box::new(self),
            op: BinOp::Mod,
            right: Box::new(rhs),
        }
    }
}

/// `-expr`
impl std_ops::Neg for Expr {
    type Output = Expr;
    fn neg(self) -> Expr {
        Expr::UnaryOp {
            op: UnaryOp::Neg,
            expr: Box::new(self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── 1. Constructor functions ──

    #[test]
    fn test_field() {
        assert!(matches!(field("email"), Expr::Identifier(s) if s == "email"));
    }

    #[test]
    fn test_col() {
        assert!(matches!(col("name"), Expr::Identifier(s) if s == "name"));
    }

    #[test]
    fn test_qualified() {
        assert!(matches!(
            qualified("users", "email"),
            Expr::QualifiedIdentifier { scope, name } if scope == "users" && name == "email"
        ));
    }

    #[test]
    fn test_qualified_col() {
        assert!(matches!(
            qualified_col("orders", "id"),
            Expr::QualifiedIdentifier { scope, name } if scope == "orders" && name == "id"
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
            Expr::BinaryOp { left, op, right } => {
                assert!(matches!(*left, Expr::Identifier(s) if s == "age"));
                assert_eq!(op, BinOp::Gt);
                assert!(matches!(*right, Expr::Literal(Literal::Int(18))));
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
            Expr::BinaryOp { right, op, .. } => {
                assert_eq!(op, BinOp::Eq);
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
}
