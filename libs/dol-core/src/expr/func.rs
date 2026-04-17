//! Function constructors that return composable [`Expr`] nodes.
//!
//! # Example
//!
//! ```rust
//! use dol_core::expr::func;
//! use dol_core::expr::field;
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
pub fn func<'a>(name: &str, args: Vec<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: name.to_string(), args }
}

// ---------------------------------------------------------------------------
// Aggregate functions
// ---------------------------------------------------------------------------

pub fn count<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "COUNT".to_string(), args: vec![expr.into()] }
}

pub fn count_star<'a>() -> Expr<'a> { Expr::CountStar }

pub fn sum<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "SUM".to_string(), args: vec![expr.into()] }
}

pub fn avg<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "AVG".to_string(), args: vec![expr.into()] }
}

pub fn min<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "MIN".to_string(), args: vec![expr.into()] }
}

pub fn max<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "MAX".to_string(), args: vec![expr.into()] }
}

// ---------------------------------------------------------------------------
// String functions
// ---------------------------------------------------------------------------

pub fn lower<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "LOWER".to_string(), args: vec![expr.into()] }
}

pub fn upper<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "UPPER".to_string(), args: vec![expr.into()] }
}

pub fn trim<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "TRIM".to_string(), args: vec![expr.into()] }
}

pub fn length<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "LENGTH".to_string(), args: vec![expr.into()] }
}

pub fn substr<'a>(expr: impl Into<Expr<'a>>, start: impl Into<Expr<'a>>, len: impl Into<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "SUBSTR".to_string(), args: vec![expr.into(), start.into(), len.into()] }
}

pub fn concat_fn<'a>(args: Vec<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "CONCAT".to_string(), args }
}

pub fn replace<'a>(expr: impl Into<Expr<'a>>, from: impl Into<Expr<'a>>, to: impl Into<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "REPLACE".to_string(), args: vec![expr.into(), from.into(), to.into()] }
}

// ---------------------------------------------------------------------------
// Date/Time functions
// ---------------------------------------------------------------------------

pub fn now<'a>() -> Expr<'a> {
    Expr::Func { name: "NOW".to_string(), args: vec![] }
}

pub fn current_date<'a>() -> Expr<'a> {
    Expr::Raw("CURRENT_DATE".to_string())
}

pub fn current_timestamp<'a>() -> Expr<'a> {
    Expr::Raw("CURRENT_TIMESTAMP".to_string())
}

// ---------------------------------------------------------------------------
// Null-handling functions
// ---------------------------------------------------------------------------

pub fn coalesce<'a>(args: Vec<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "COALESCE".to_string(), args }
}

pub fn nullif<'a>(expr1: impl Into<Expr<'a>>, expr2: impl Into<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "NULLIF".to_string(), args: vec![expr1.into(), expr2.into()] }
}

// ---------------------------------------------------------------------------
// Window functions (chain with .over().partition_by().build())
// ---------------------------------------------------------------------------

pub fn row_number<'a>() -> Expr<'a> {
    Expr::Func { name: "ROW_NUMBER".to_string(), args: vec![] }
}

pub fn rank<'a>() -> Expr<'a> {
    Expr::Func { name: "RANK".to_string(), args: vec![] }
}

pub fn dense_rank<'a>() -> Expr<'a> {
    Expr::Func { name: "DENSE_RANK".to_string(), args: vec![] }
}

pub fn ntile<'a>(n: impl Into<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "NTILE".to_string(), args: vec![n.into()] }
}

pub fn lag<'a>(expr: impl Into<Expr<'a>>, offset: Option<Expr<'a>>, default: Option<Expr<'a>>) -> Expr<'a> {
    let mut args = vec![expr.into()];
    if let Some(o) = offset { args.push(o); }
    if let Some(d) = default { args.push(d); }
    Expr::Func { name: "LAG".to_string(), args }
}

pub fn lead<'a>(expr: impl Into<Expr<'a>>, offset: Option<Expr<'a>>, default: Option<Expr<'a>>) -> Expr<'a> {
    let mut args = vec![expr.into()];
    if let Some(o) = offset { args.push(o); }
    if let Some(d) = default { args.push(d); }
    Expr::Func { name: "LEAD".to_string(), args }
}

pub fn first_value<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "FIRST_VALUE".to_string(), args: vec![expr.into()] }
}

pub fn last_value<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "LAST_VALUE".to_string(), args: vec![expr.into()] }
}

// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

pub fn abs<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: "ABS".to_string(), args: vec![expr.into()] }
}

pub fn round<'a>(expr: impl Into<Expr<'a>>, precision: Option<Expr<'a>>) -> Expr<'a> {
    let mut args = vec![expr.into()];
    if let Some(p) = precision { args.push(p); }
    Expr::Func { name: "ROUND".to_string(), args }
}
