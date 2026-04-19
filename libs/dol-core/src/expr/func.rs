//! Function names — the single catalogue of every function DOL can express.
//!
//! [`FuncName`] is a flat enum: every well-known function is a unit variant,
//! and [`FuncName::Custom`] is the escape hatch for backend-specific or
//! user-defined functions.

use super::Expr;

// ---------------------------------------------------------------------------
// FuncName — the single function identifier enum
// ---------------------------------------------------------------------------

/// Identifies a function in a DOL expression.
///
/// Every well-known, backend-agnostic function has its own unit variant.
/// [`Custom`](Self::Custom) is the escape hatch for backend-specific or
/// user-defined function names.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FuncName {
    // ── Aggregate ────────────────────────────────────────────────────────
    Count,
    CountDistinct,
    Sum,
    Avg,
    Min,
    Max,
    Median,
    StdDev,
    Variance,
    ArrayAgg,
    StringAgg,
    JsonAgg,
    BoolAnd,
    BoolOr,
    First,
    Last,

    // ── String ───────────────────────────────────────────────────────────
    Lower,
    Upper,
    Trim,
    LTrim,
    RTrim,
    Length,
    CharLength,
    OctetLength,
    Substr,
    Left,
    Right,
    Concat,
    ConcatWs,
    Replace,
    Reverse,
    Repeat,
    PadLeft,
    PadRight,
    Position,
    Initcap,
    Ascii,
    Chr,
    Md5,
    Sha256,
    Base64Encode,
    Base64Decode,
    RegexReplace,
    RegexExtract,
    Split,
    SplitPart,
    Format,
    StartsWith,
    Contains,
    ToHex,

    // ── Numeric / Math ───────────────────────────────────────────────────
    Abs,
    Ceil,
    Floor,
    Round,
    Trunc,
    Sign,
    Power,
    Sqrt,
    Cbrt,
    Exp,
    Ln,
    Log,
    Log2,
    Log10,
    Pi,
    Degrees,
    Radians,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Atan2,
    Sinh,
    Cosh,
    Tanh,
    Factorial,
    Gcd,
    Lcm,
    Random,
    Greatest,
    Least,

    // ── Date / Time ───────────────────────────────────────────────────────
    Now,
    CurrentDate,
    CurrentTime,
    CurrentTimestamp,
    DatePart,
    DateTrunc,
    Extract,
    DateAdd,
    DateSub,
    DateDiff,
    Age,
    ToDate,
    ToTimestamp,
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
    DayOfWeek,
    DayOfYear,
    WeekOfYear,
    Quarter,
    MakeDate,
    MakeTime,
    MakeTimestamp,
    EpochToTimestamp,
    TimestampToEpoch,

    // ── Null-handling ─────────────────────────────────────────────────────
    Coalesce,
    NullIf,
    IfNull,

    // ── Type conversion ───────────────────────────────────────────────────
    TypeOf,
    ToText,
    ToInt,
    ToFloat,
    ToBool,

    // ── JSON / Document ───────────────────────────────────────────────────
    JsonGet,
    JsonGetText,
    JsonPath,
    JsonPathText,
    JsonHasKey,
    JsonHasAnyKey,
    JsonHasAllKeys,
    JsonSet,
    JsonInsert,
    JsonRemove,
    JsonReplace,
    JsonMergePatch,
    JsonArray,
    JsonObject,
    JsonArrayLength,
    JsonKeys,
    JsonValues,
    JsonTypeof,

    // ── Array / Collection ────────────────────────────────────────────────
    ArrayLength,
    ArrayPosition,
    ArrayAppend,
    ArrayPrepend,
    ArrayRemove,
    ArrayCat,
    ArrayDistinct,
    ArraySort,
    ArrayReverse,
    ArraySlice,
    ArrayFlatten,
    Unnest,
    ArrayToString,
    StringToArray,

    // ── Object / Map ─────────────────────────────────────────────────────
    MapMerge,
    MapGet,
    MapKeys,
    MapValues,
    MapContainsKey,
    MapRemoveKey,

    // ── Range operations ──────────────────────────────────────────────────
    RangeContains,
    RangeContainedBy,
    RangeOverlap,
    RangeLower,
    RangeUpper,
    RangeIsEmpty,

    // ── Window / Ranking ─────────────────────────────────────────────────
    RowNumber,
    Rank,
    DenseRank,
    NTile,
    Lag,
    Lead,
    FirstValue,
    LastValue,
    NthValue,
    CumeDist,
    PercentRank,

    // ── UUID ─────────────────────────────────────────────────────────────
    GenRandomUuid,

    // ── Geo / Spatial ─────────────────────────────────────────────────────
    StContains,
    StIntersects,
    StWithin,
    StArea,
    StLength,
    StDistance,
    StBuffer,
    StCentroid,
    StAsText,
    StGeomFromText,

    // ── Hashing / Encoding ────────────────────────────────────────────────
    Hash,
    Crc32,
    HexEncode,
    HexDecode,

    // ── Escape hatch ─────────────────────────────────────────────────────
    /// A custom or backend-specific function name.
    Custom(String),
}

// ---------------------------------------------------------------------------
// Generic function builders
// ---------------------------------------------------------------------------

/// Build a function-call expression from a custom name string.
pub fn func<'a>(name: &str, args: Vec<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: FuncName::Custom(name.to_string()), args }
}

/// Build a function-call expression from a well-known [`FuncName`] variant.
fn known<'a>(name: FuncName, args: Vec<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name, args }
}

// ---------------------------------------------------------------------------
// Aggregate functions
// ---------------------------------------------------------------------------

pub fn count<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Count, vec![expr.into()])
}

pub fn count_star<'a>() -> Expr<'a> { Expr::CountStar }

pub fn sum<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Sum, vec![expr.into()])
}

pub fn avg<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Avg, vec![expr.into()])
}

pub fn min<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Min, vec![expr.into()])
}

pub fn max<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Max, vec![expr.into()])
}

// ---------------------------------------------------------------------------
// String functions
// ---------------------------------------------------------------------------

pub fn lower<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Lower, vec![expr.into()])
}

pub fn upper<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Upper, vec![expr.into()])
}

pub fn trim<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Trim, vec![expr.into()])
}

pub fn length<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Length, vec![expr.into()])
}

pub fn substr<'a>(expr: impl Into<Expr<'a>>, start: impl Into<Expr<'a>>, len: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Substr, vec![expr.into(), start.into(), len.into()])
}

pub fn concat_fn<'a>(args: Vec<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Concat, args)
}

pub fn replace<'a>(expr: impl Into<Expr<'a>>, from: impl Into<Expr<'a>>, to: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Replace, vec![expr.into(), from.into(), to.into()])
}

// ---------------------------------------------------------------------------
// Date/Time functions
// ---------------------------------------------------------------------------

pub fn now<'a>() -> Expr<'a> {
    known(FuncName::Now, vec![])
}

pub fn current_date<'a>() -> Expr<'a> {
    known(FuncName::CurrentDate, vec![])
}

pub fn current_timestamp<'a>() -> Expr<'a> {
    known(FuncName::CurrentTimestamp, vec![])
}

// ---------------------------------------------------------------------------
// Null-handling functions
// ---------------------------------------------------------------------------

pub fn coalesce<'a>(args: Vec<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Coalesce, args)
}

pub fn nullif<'a>(expr1: impl Into<Expr<'a>>, expr2: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::NullIf, vec![expr1.into(), expr2.into()])
}

// ---------------------------------------------------------------------------
// Window functions
// ---------------------------------------------------------------------------

pub fn row_number<'a>() -> Expr<'a> {
    known(FuncName::RowNumber, vec![])
}

pub fn rank<'a>() -> Expr<'a> {
    known(FuncName::Rank, vec![])
}

pub fn dense_rank<'a>() -> Expr<'a> {
    known(FuncName::DenseRank, vec![])
}

pub fn ntile<'a>(n: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::NTile, vec![n.into()])
}

pub fn lag<'a>(expr: impl Into<Expr<'a>>, offset: Option<Expr<'a>>, default: Option<Expr<'a>>) -> Expr<'a> {
    let mut args = vec![expr.into()];
    if let Some(o) = offset { args.push(o); }
    if let Some(d) = default { args.push(d); }
    known(FuncName::Lag, args)
}

pub fn lead<'a>(expr: impl Into<Expr<'a>>, offset: Option<Expr<'a>>, default: Option<Expr<'a>>) -> Expr<'a> {
    let mut args = vec![expr.into()];
    if let Some(o) = offset { args.push(o); }
    if let Some(d) = default { args.push(d); }
    known(FuncName::Lead, args)
}

pub fn first_value<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::FirstValue, vec![expr.into()])
}

pub fn last_value<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::LastValue, vec![expr.into()])
}

// ---------------------------------------------------------------------------
// Math / Numeric functions
// ---------------------------------------------------------------------------

pub fn abs<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Abs, vec![expr.into()])
}

pub fn round<'a>(expr: impl Into<Expr<'a>>, precision: Option<Expr<'a>>) -> Expr<'a> {
    let mut args = vec![expr.into()];
    if let Some(p) = precision { args.push(p); }
    known(FuncName::Round, args)
}

pub fn sqrt<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Sqrt, vec![expr.into()])
}

pub fn cbrt<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Cbrt, vec![expr.into()])
}

pub fn factorial<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Factorial, vec![expr.into()])
}

pub fn greatest<'a>(args: Vec<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Greatest, args)
}

pub fn least<'a>(args: Vec<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Least, args)
}

// ---------------------------------------------------------------------------
// JSON functions
// ---------------------------------------------------------------------------

pub fn json_get<'a>(doc: impl Into<Expr<'a>>, key: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::JsonGet, vec![doc.into(), key.into()])
}

pub fn json_has_key<'a>(doc: impl Into<Expr<'a>>, key: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::JsonHasKey, vec![doc.into(), key.into()])
}

// ---------------------------------------------------------------------------
// Array functions
// ---------------------------------------------------------------------------

pub fn array_append<'a>(arr: impl Into<Expr<'a>>, elem: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::ArrayAppend, vec![arr.into(), elem.into()])
}

pub fn array_prepend<'a>(elem: impl Into<Expr<'a>>, arr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::ArrayPrepend, vec![elem.into(), arr.into()])
}

pub fn array_length<'a>(arr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::ArrayLength, vec![arr.into()])
}

pub fn unnest<'a>(arr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::Unnest, vec![arr.into()])
}

// ---------------------------------------------------------------------------
// Geo / Spatial functions
// ---------------------------------------------------------------------------

pub fn st_distance<'a>(a: impl Into<Expr<'a>>, b: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncName::StDistance, vec![a.into(), b.into()])
}
