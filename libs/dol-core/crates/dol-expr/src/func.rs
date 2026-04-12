//! Function constructors that return composable [`Expr`] nodes.
//!
//! # Example
//!
//! ```rust
//! use dol_expr::func;
//! use dol_expr::field;
//!
//! let expr = func::lower(field("email"));
//! let expr = func::coalesce(vec![field("name"), field("email")]);
//! let expr = func::count_star();
//! ```

use super::Expr;

// ---------------------------------------------------------------------------
// Generic function builder
// ---------------------------------------------------------------------------

/// Create a generic function call: `name(args...)`.
pub fn func(name: &str, args: Vec<Expr>) -> Expr {
    Expr::Func {
        name: name.to_string(),
        args,
    }
}

// ---------------------------------------------------------------------------
// Aggregate functions
// ---------------------------------------------------------------------------

/// `COUNT(expr)`
pub fn count(expr: impl Into<Expr>) -> Expr {
    Expr::Func {
        name: "COUNT".to_string(),
        args: vec![expr.into()],
    }
}

/// `COUNT(*)`
pub fn count_star() -> Expr {
    Expr::CountStar
}

/// `SUM(expr)`
pub fn sum(expr: impl Into<Expr>) -> Expr {
    Expr::Func {
        name: "SUM".to_string(),
        args: vec![expr.into()],
    }
}

/// `AVG(expr)`
pub fn avg(expr: impl Into<Expr>) -> Expr {
    Expr::Func {
        name: "AVG".to_string(),
        args: vec![expr.into()],
    }
}

/// `MIN(expr)`
pub fn min(expr: impl Into<Expr>) -> Expr {
    Expr::Func {
        name: "MIN".to_string(),
        args: vec![expr.into()],
    }
}

/// `MAX(expr)`
pub fn max(expr: impl Into<Expr>) -> Expr {
    Expr::Func {
        name: "MAX".to_string(),
        args: vec![expr.into()],
    }
}

// ---------------------------------------------------------------------------
// String functions
// ---------------------------------------------------------------------------

/// `LOWER(expr)`
pub fn lower(expr: impl Into<Expr>) -> Expr {
    Expr::Func {
        name: "LOWER".to_string(),
        args: vec![expr.into()],
    }
}

/// `UPPER(expr)`
pub fn upper(expr: impl Into<Expr>) -> Expr {
    Expr::Func {
        name: "UPPER".to_string(),
        args: vec![expr.into()],
    }
}

/// `TRIM(expr)`
pub fn trim(expr: impl Into<Expr>) -> Expr {
    Expr::Func {
        name: "TRIM".to_string(),
        args: vec![expr.into()],
    }
}

/// `LENGTH(expr)`
pub fn length(expr: impl Into<Expr>) -> Expr {
    Expr::Func {
        name: "LENGTH".to_string(),
        args: vec![expr.into()],
    }
}

/// `SUBSTR(expr, start, length)`
pub fn substr(expr: impl Into<Expr>, start: impl Into<Expr>, len: impl Into<Expr>) -> Expr {
    Expr::Func {
        name: "SUBSTR".to_string(),
        args: vec![expr.into(), start.into(), len.into()],
    }
}

/// `CONCAT(args...)` — generic concat function.
///
/// For dialect-aware string concatenation that respects `||` vs `CONCAT()` vs `+`,
/// use [`Expr::concat()`] instead.
pub fn concat_fn(args: Vec<Expr>) -> Expr {
    Expr::Func {
        name: "CONCAT".to_string(),
        args,
    }
}

/// `REPLACE(expr, from, to)`
pub fn replace(expr: impl Into<Expr>, from: impl Into<Expr>, to: impl Into<Expr>) -> Expr {
    Expr::Func {
        name: "REPLACE".to_string(),
        args: vec![expr.into(), from.into(), to.into()],
    }
}

// ---------------------------------------------------------------------------
// Date/Time functions
// ---------------------------------------------------------------------------

/// `NOW()`
pub fn now() -> Expr {
    Expr::Func {
        name: "NOW".to_string(),
        args: vec![],
    }
}

/// `CURRENT_DATE`
pub fn current_date() -> Expr {
    Expr::Raw("CURRENT_DATE".to_string())
}

/// `CURRENT_TIMESTAMP`
pub fn current_timestamp() -> Expr {
    Expr::Raw("CURRENT_TIMESTAMP".to_string())
}

// ---------------------------------------------------------------------------
// Null-handling functions
// ---------------------------------------------------------------------------

/// `COALESCE(args...)`
pub fn coalesce(args: Vec<Expr>) -> Expr {
    Expr::Func {
        name: "COALESCE".to_string(),
        args,
    }
}

/// `NULLIF(expr1, expr2)`
pub fn nullif(expr1: impl Into<Expr>, expr2: impl Into<Expr>) -> Expr {
    Expr::Func {
        name: "NULLIF".to_string(),
        args: vec![expr1.into(), expr2.into()],
    }
}

// ---------------------------------------------------------------------------
// Window functions (chain with .over().partition_by().build())
// ---------------------------------------------------------------------------

/// `ROW_NUMBER()`
pub fn row_number() -> Expr {
    Expr::Func {
        name: "ROW_NUMBER".to_string(),
        args: vec![],
    }
}

/// `RANK()`
pub fn rank() -> Expr {
    Expr::Func {
        name: "RANK".to_string(),
        args: vec![],
    }
}

/// `DENSE_RANK()`
pub fn dense_rank() -> Expr {
    Expr::Func {
        name: "DENSE_RANK".to_string(),
        args: vec![],
    }
}

/// `NTILE(n)`
pub fn ntile(n: impl Into<Expr>) -> Expr {
    Expr::Func {
        name: "NTILE".to_string(),
        args: vec![n.into()],
    }
}

/// `LAG(expr [, offset [, default]])`
pub fn lag(expr: impl Into<Expr>, offset: Option<Expr>, default: Option<Expr>) -> Expr {
    let mut args = vec![expr.into()];
    if let Some(o) = offset {
        args.push(o);
    }
    if let Some(d) = default {
        args.push(d);
    }
    Expr::Func {
        name: "LAG".to_string(),
        args,
    }
}

/// `LEAD(expr [, offset [, default]])`
pub fn lead(expr: impl Into<Expr>, offset: Option<Expr>, default: Option<Expr>) -> Expr {
    let mut args = vec![expr.into()];
    if let Some(o) = offset {
        args.push(o);
    }
    if let Some(d) = default {
        args.push(d);
    }
    Expr::Func {
        name: "LEAD".to_string(),
        args,
    }
}

/// `FIRST_VALUE(expr)`
pub fn first_value(expr: impl Into<Expr>) -> Expr {
    Expr::Func {
        name: "FIRST_VALUE".to_string(),
        args: vec![expr.into()],
    }
}

/// `LAST_VALUE(expr)`
pub fn last_value(expr: impl Into<Expr>) -> Expr {
    Expr::Func {
        name: "LAST_VALUE".to_string(),
        args: vec![expr.into()],
    }
}

// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

/// `ABS(expr)`
pub fn abs(expr: impl Into<Expr>) -> Expr {
    Expr::Func {
        name: "ABS".to_string(),
        args: vec![expr.into()],
    }
}

/// `ROUND(expr [, precision])`
pub fn round(expr: impl Into<Expr>, precision: Option<Expr>) -> Expr {
    let mut args = vec![expr.into()];
    if let Some(p) = precision {
        args.push(p);
    }
    Expr::Func {
        name: "ROUND".to_string(),
        args,
    }
}
