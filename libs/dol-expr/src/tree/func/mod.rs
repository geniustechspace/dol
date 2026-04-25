//! Function builders — ergonomic constructors for well-known DOL functions.
//!
//! Submodules:
//! - [`def`]      — `FuncDef`/`OpDef` re-export barrel for stable import paths
//! - [`meta`]     — function metadata: signatures, arity, kind
//! - [`registry`] — built-in function registry

pub mod def;
pub mod meta;
pub mod registry;

use super::Expr;
use meta::DolFunc;

// ---------------------------------------------------------------------------
// Generic function builders
// ---------------------------------------------------------------------------

/// Build a function-call expression from any name string.
pub fn func<'a>(name: &str, args: Vec<Expr<'a>>) -> Expr<'a> {
    Expr::Func {
        name: super::FuncDef::custom(name),
        args,
    }
}

/// Internal helper to build a function-call expression from a well-known
/// [`FuncDef`] (zero heap allocation for the name).
fn known_def<'a>(def: super::FuncDef, args: Vec<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: def, args }
}

// ---------------------------------------------------------------------------
// Aggregate functions
// ---------------------------------------------------------------------------

pub fn count<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Count::def(), vec![expr.into()])
}

pub fn count_star<'a>() -> Expr<'a> {
    Expr::CountStar
}

pub fn sum<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Sum::def(), vec![expr.into()])
}

pub fn avg<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Avg::def(), vec![expr.into()])
}

pub fn min<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Min::def(), vec![expr.into()])
}

pub fn max<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Max::def(), vec![expr.into()])
}

// ---------------------------------------------------------------------------
// String functions
// ---------------------------------------------------------------------------

pub fn lower<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Lower::def(), vec![expr.into()])
}

pub fn upper<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Upper::def(), vec![expr.into()])
}

pub fn trim<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Trim::def(), vec![expr.into()])
}

pub fn length<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Length::def(), vec![expr.into()])
}

pub fn substr<'a>(
    expr: impl Into<Expr<'a>>,
    start: impl Into<Expr<'a>>,
    len: impl Into<Expr<'a>>,
) -> Expr<'a> {
    known_def(
        registry::Substr::def(),
        vec![expr.into(), start.into(), len.into()],
    )
}

pub fn concat_fn<'a>(args: Vec<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Concat::def(), args)
}

pub fn replace<'a>(
    expr: impl Into<Expr<'a>>,
    from: impl Into<Expr<'a>>,
    to: impl Into<Expr<'a>>,
) -> Expr<'a> {
    known_def(
        registry::Replace::def(),
        vec![expr.into(), from.into(), to.into()],
    )
}

// ---------------------------------------------------------------------------
// Date/Time functions
// ---------------------------------------------------------------------------

pub fn now<'a>() -> Expr<'a> {
    known_def(registry::Now::def(), vec![])
}

pub fn current_date<'a>() -> Expr<'a> {
    known_def(registry::CurrentDate::def(), vec![])
}

pub fn current_timestamp<'a>() -> Expr<'a> {
    known_def(registry::CurrentTimestamp::def(), vec![])
}

// ---------------------------------------------------------------------------
// Null-handling functions
// ---------------------------------------------------------------------------

pub fn coalesce<'a>(args: Vec<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Coalesce::def(), args)
}

pub fn nullif<'a>(expr1: impl Into<Expr<'a>>, expr2: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Nullif::def(), vec![expr1.into(), expr2.into()])
}

// ---------------------------------------------------------------------------
// Window functions
// ---------------------------------------------------------------------------

pub fn row_number<'a>() -> Expr<'a> {
    known_def(registry::RowNumber::def(), vec![])
}

pub fn rank<'a>() -> Expr<'a> {
    known_def(registry::Rank::def(), vec![])
}

pub fn dense_rank<'a>() -> Expr<'a> {
    known_def(registry::DenseRank::def(), vec![])
}

pub fn ntile<'a>(n: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Ntile::def(), vec![n.into()])
}

pub fn lag<'a>(
    expr: impl Into<Expr<'a>>,
    offset: Option<Expr<'a>>,
    default: Option<Expr<'a>>,
) -> Expr<'a> {
    let mut args = vec![expr.into()];
    if let Some(o) = offset {
        args.push(o);
    }
    if let Some(d) = default {
        args.push(d);
    }
    known_def(registry::Lag::def(), args)
}

pub fn lead<'a>(
    expr: impl Into<Expr<'a>>,
    offset: Option<Expr<'a>>,
    default: Option<Expr<'a>>,
) -> Expr<'a> {
    let mut args = vec![expr.into()];
    if let Some(o) = offset {
        args.push(o);
    }
    if let Some(d) = default {
        args.push(d);
    }
    known_def(registry::Lead::def(), args)
}

pub fn first_value<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::FirstValue::def(), vec![expr.into()])
}

pub fn last_value<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::LastValue::def(), vec![expr.into()])
}

// ---------------------------------------------------------------------------
// Math / Numeric functions
// ---------------------------------------------------------------------------

pub fn abs<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Abs::def(), vec![expr.into()])
}

pub fn round<'a>(expr: impl Into<Expr<'a>>, precision: Option<Expr<'a>>) -> Expr<'a> {
    let mut args = vec![expr.into()];
    if let Some(p) = precision {
        args.push(p);
    }
    known_def(registry::Round::def(), args)
}

pub fn sqrt<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Sqrt::def(), vec![expr.into()])
}

pub fn cbrt<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Cbrt::def(), vec![expr.into()])
}

pub fn factorial<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Factorial::def(), vec![expr.into()])
}

pub fn greatest<'a>(args: Vec<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Greatest::def(), args)
}

pub fn least<'a>(args: Vec<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Least::def(), args)
}

// ---------------------------------------------------------------------------
// JSON functions
// ---------------------------------------------------------------------------

pub fn json_get<'a>(doc: impl Into<Expr<'a>>, key: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::JsonGet::def(), vec![doc.into(), key.into()])
}

pub fn json_has_key<'a>(doc: impl Into<Expr<'a>>, key: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::JsonHasKey::def(), vec![doc.into(), key.into()])
}

// ---------------------------------------------------------------------------
// Array functions
// ---------------------------------------------------------------------------

pub fn array_append<'a>(arr: impl Into<Expr<'a>>, elem: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::ArrayAppend::def(), vec![arr.into(), elem.into()])
}

pub fn array_prepend<'a>(elem: impl Into<Expr<'a>>, arr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::ArrayPrepend::def(), vec![elem.into(), arr.into()])
}

pub fn array_length<'a>(arr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::ArrayLength::def(), vec![arr.into()])
}

pub fn unnest<'a>(arr: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::Unnest::def(), vec![arr.into()])
}

// ---------------------------------------------------------------------------
// Geo / Spatial functions
// ---------------------------------------------------------------------------

pub fn st_distance<'a>(a: impl Into<Expr<'a>>, b: impl Into<Expr<'a>>) -> Expr<'a> {
    known_def(registry::StDistance::def(), vec![a.into(), b.into()])
}
