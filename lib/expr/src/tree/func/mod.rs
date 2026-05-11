//! Function builders and DSL entry points for `tree::Expr`.
//!
//! # Using well-known functions
//!
//! Most well-known functions are available as free functions re-exported from
//! [`registry`].  Import whichever you need:
//!
//! ```ignore
//! use dol_expr::tree::func::{lower, count, coalesce, row_number};
//! ```
//!
//! For functions whose builder needs optional trailing arguments (see below),
//! hand-rolled helpers are provided directly in this module.
//!
//! # Using the universal call path
//!
//! Every registry struct exposes `Foo::call(args)` from the [`DolFunc`] trait.
//! Use this for functions without a builder, for the minority arity variants,
//! or anywhere you want an explicit, uniform call site:
//!
//! ```ignore
//! use dol_expr::tree::func::registry::{Substr, Round, PadLeft};
//!
//! // 2-arg substr (start only, no length)
//! let e = Substr::call(vec![field("name"), lit(3i64)]);
//!
//! // round with no precision
//! let e = Round::call(vec![field("score")]);
//! ```
//!
//! # Custom / unknown functions
//!
//! Use [`func`] for any function name that has no registry entry:
//!
//! ```ignore
//! let e = func("my_backend_fn", vec![field("x"), lit(42i64)]);
//! ```

pub mod meta;
pub mod registry;

pub use meta::{Arity, ArityError, DolFunc, FuncDef, FuncKind};

// Re-export all registry-generated builders at the `func` module level so
// callers can write `func::lower(s)` rather than `func::registry::lower(s)`.
#[rustfmt::skip]
pub use registry::{
    // aggregates
    avg, count, max, min, string_agg, sum,
    // strings
    concat, contains, left, length, lower, position, repeat, replace,
    reverse, right, split, split_part, starts_with, substr, trim, upper,
    // date / time
    current_date, current_time, current_timestamp, day, date_part,
    date_trunc, extract, hour, minute, month, now, second, year,
    // null-handling
    coalesce, ifnull, nullif,
    // type conversion
    to_bool, to_float, to_int, to_text,
    // window
    dense_rank, first_value, last_value, nth_value, ntile, rank, row_number,
    // math
    abs, cbrt, ceil, factorial, floor, greatest, least, pi, power, random, sqrt,
    // json
    json_get, json_get_text, json_has_key,
    // array
    array_append, array_cat, array_length, array_position,
    array_prepend, array_remove, unnest,
    // geo
    st_contains, st_distance, st_intersects, st_within,
    // misc
    gen_random_uuid, hash,
};

use alloc::vec;
use alloc::vec::Vec;

use super::Expr;

// ─────────────────────────────────────────────────────────────────────────────
// Custom function call
// ─────────────────────────────────────────────────────────────────────────────

/// Build a function-call [`Expr`] for any function name not in the registry.
///
/// For well-known functions prefer the typed builders above or
/// `registry::Foo::call(args)` — they carry arity information and are
/// stable across renames.
pub fn func<'a>(name: &str, args: Vec<Expr<'a>>) -> Expr<'a> {
    Expr::Func {
        name: FuncDef::custom(name),
        args,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Special-case: COUNT(*)
// ─────────────────────────────────────────────────────────────────────────────

/// `COUNT(*)` — counts all rows regardless of nulls.
///
/// This is a distinct [`Expr`] variant, not a function call; no registry entry
/// exists for it.
pub fn count_star<'a>() -> Expr<'a> {
    Expr::CountStar
}

// ─────────────────────────────────────────────────────────────────────────────
// Optional-arg builders (cannot be expressed through define_func!)
// ─────────────────────────────────────────────────────────────────────────────

/// `ROUND(expr [, precision])`.
///
/// Omit `precision` to round to the nearest integer.
pub fn round<'a>(expr: impl Into<Expr<'a>>, precision: Option<Expr<'a>>) -> Expr<'a> {
    let mut args = vec![expr.into()];
    if let Some(p) = precision {
        args.push(p);
    }
    registry::Round::call(args)
}

/// `LAG(expr [, offset [, default]])`.
///
/// - `offset`  — how many rows back (default: 1).
/// - `default` — value to return when the offset reaches beyond the partition.
pub fn lag<'a>(
    expr: impl Into<Expr<'a>>,
    offset: Option<Expr<'a>>,
    default: Option<Expr<'a>>,
) -> Expr<'a> {
    let mut args = vec![expr.into()];
    if let Some(o) = offset {
        args.push(o);
        if let Some(d) = default {
            args.push(d);
        }
    }
    registry::Lag::call(args)
}

/// `LEAD(expr [, offset [, default]])`.
///
/// - `offset`  — how many rows forward (default: 1).
/// - `default` — value to return when the offset reaches beyond the partition.
pub fn lead<'a>(
    expr: impl Into<Expr<'a>>,
    offset: Option<Expr<'a>>,
    default: Option<Expr<'a>>,
) -> Expr<'a> {
    let mut args = vec![expr.into()];
    if let Some(o) = offset {
        args.push(o);
        if let Some(d) = default {
            args.push(d);
        }
    }
    registry::Lead::call(args)
}
