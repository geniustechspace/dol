//! Function identifiers — the extensible catalogue of every function DOL can express.
//!
//! [`FuncId`] is a string-keyed newtype with well-known constants for every
//! standard function. Custom or backend-specific functions use
//! [`FuncId::new()`] — no enum variant needed.

use super::Expr;
use core::fmt;

// ---------------------------------------------------------------------------
// FuncId — extensible function identifier
// ---------------------------------------------------------------------------

/// Identifies a function in a DOL expression.
///
/// Every well-known, backend-agnostic function has a corresponding `&str`
/// constant (e.g. [`FuncId::COUNT`], [`FuncId::SUM`]).
///
/// Custom or backend-specific functions can be created via [`FuncId::new()`]
/// without modifying this type — that is the extensibility advantage over an
/// enum.
///
/// # Matching
///
/// Backends and consumers match on [`FuncId::as_str()`]:
///
/// ```ignore
/// match func_id.as_str() {
///     FuncId::COUNT => ...,
///     FuncId::SUM   => ...,
///     unknown       => Err(Unsupported(...)),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FuncId(Box<str>);

impl FuncId {
    /// Create a function identifier from any string.
    pub fn new(name: impl Into<Box<str>>) -> Self {
        Self(name.into())
    }

    /// Return the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    // ── Aggregate ────────────────────────────────────────────────────────
    pub const COUNT: &str = "count";
    pub const COUNT_DISTINCT: &str = "count_distinct";
    pub const SUM: &str = "sum";
    pub const AVG: &str = "avg";
    pub const MIN: &str = "min";
    pub const MAX: &str = "max";
    pub const MEDIAN: &str = "median";
    pub const STDDEV: &str = "stddev";
    pub const VARIANCE: &str = "variance";
    pub const ARRAY_AGG: &str = "array_agg";
    pub const STRING_AGG: &str = "string_agg";
    pub const JSON_AGG: &str = "json_agg";
    pub const BOOL_AND: &str = "bool_and";
    pub const BOOL_OR: &str = "bool_or";
    pub const FIRST: &str = "first";
    pub const LAST: &str = "last";

    // ── String ───────────────────────────────────────────────────────────
    pub const LOWER: &str = "lower";
    pub const UPPER: &str = "upper";
    pub const TRIM: &str = "trim";
    pub const LTRIM: &str = "ltrim";
    pub const RTRIM: &str = "rtrim";
    pub const LENGTH: &str = "length";
    pub const CHAR_LENGTH: &str = "char_length";
    pub const OCTET_LENGTH: &str = "octet_length";
    pub const SUBSTR: &str = "substr";
    pub const LEFT: &str = "left";
    pub const RIGHT: &str = "right";
    pub const CONCAT: &str = "concat";
    pub const CONCAT_WS: &str = "concat_ws";
    pub const REPLACE: &str = "replace";
    pub const REVERSE: &str = "reverse";
    pub const REPEAT: &str = "repeat";
    pub const PAD_LEFT: &str = "pad_left";
    pub const PAD_RIGHT: &str = "pad_right";
    pub const POSITION: &str = "position";
    pub const INITCAP: &str = "initcap";
    pub const ASCII: &str = "ascii";
    pub const CHR: &str = "chr";
    pub const MD5: &str = "md5";
    pub const SHA256: &str = "sha256";
    pub const BASE64_ENCODE: &str = "base64_encode";
    pub const BASE64_DECODE: &str = "base64_decode";
    pub const REGEX_REPLACE: &str = "regex_replace";
    pub const REGEX_EXTRACT: &str = "regex_extract";
    pub const SPLIT: &str = "split";
    pub const SPLIT_PART: &str = "split_part";
    pub const FORMAT: &str = "format";
    pub const STARTS_WITH: &str = "starts_with";
    pub const CONTAINS: &str = "contains";
    pub const TO_HEX: &str = "to_hex";

    // ── Numeric / Math ───────────────────────────────────────────────────
    pub const ABS: &str = "abs";
    pub const CEIL: &str = "ceil";
    pub const FLOOR: &str = "floor";
    pub const ROUND: &str = "round";
    pub const TRUNC: &str = "trunc";
    pub const SIGN: &str = "sign";
    pub const POWER: &str = "power";
    pub const SQRT: &str = "sqrt";
    pub const CBRT: &str = "cbrt";
    pub const EXP: &str = "exp";
    pub const LN: &str = "ln";
    pub const LOG: &str = "log";
    pub const LOG2: &str = "log2";
    pub const LOG10: &str = "log10";
    pub const PI: &str = "pi";
    pub const DEGREES: &str = "degrees";
    pub const RADIANS: &str = "radians";
    pub const SIN: &str = "sin";
    pub const COS: &str = "cos";
    pub const TAN: &str = "tan";
    pub const ASIN: &str = "asin";
    pub const ACOS: &str = "acos";
    pub const ATAN: &str = "atan";
    pub const ATAN2: &str = "atan2";
    pub const SINH: &str = "sinh";
    pub const COSH: &str = "cosh";
    pub const TANH: &str = "tanh";
    pub const FACTORIAL: &str = "factorial";
    pub const GCD: &str = "gcd";
    pub const LCM: &str = "lcm";
    pub const RANDOM: &str = "random";
    pub const GREATEST: &str = "greatest";
    pub const LEAST: &str = "least";

    // ── Date / Time ──────────────────────────────────────────────────────
    pub const NOW: &str = "now";
    pub const CURRENT_DATE: &str = "current_date";
    pub const CURRENT_TIME: &str = "current_time";
    pub const CURRENT_TIMESTAMP: &str = "current_timestamp";
    pub const DATE_PART: &str = "date_part";
    pub const DATE_TRUNC: &str = "date_trunc";
    pub const EXTRACT: &str = "extract";
    pub const DATE_ADD: &str = "date_add";
    pub const DATE_SUB: &str = "date_sub";
    pub const DATE_DIFF: &str = "date_diff";
    pub const AGE: &str = "age";
    pub const TO_DATE: &str = "to_date";
    pub const TO_TIMESTAMP: &str = "to_timestamp";
    pub const YEAR: &str = "year";
    pub const MONTH: &str = "month";
    pub const DAY: &str = "day";
    pub const HOUR: &str = "hour";
    pub const MINUTE: &str = "minute";
    pub const SECOND: &str = "second";
    pub const DAY_OF_WEEK: &str = "day_of_week";
    pub const DAY_OF_YEAR: &str = "day_of_year";
    pub const WEEK_OF_YEAR: &str = "week_of_year";
    pub const QUARTER: &str = "quarter";
    pub const MAKE_DATE: &str = "make_date";
    pub const MAKE_TIME: &str = "make_time";
    pub const MAKE_TIMESTAMP: &str = "make_timestamp";
    pub const EPOCH_TO_TIMESTAMP: &str = "epoch_to_timestamp";
    pub const TIMESTAMP_TO_EPOCH: &str = "timestamp_to_epoch";

    // ── Null-handling ────────────────────────────────────────────────────
    pub const COALESCE: &str = "coalesce";
    pub const NULLIF: &str = "nullif";
    pub const IFNULL: &str = "ifnull";

    // ── Type conversion ──────────────────────────────────────────────────
    pub const TYPEOF: &str = "typeof";
    pub const TO_TEXT: &str = "to_text";
    pub const TO_INT: &str = "to_int";
    pub const TO_FLOAT: &str = "to_float";
    pub const TO_BOOL: &str = "to_bool";

    // ── JSON / Document ──────────────────────────────────────────────────
    pub const JSON_GET: &str = "json_get";
    pub const JSON_GET_TEXT: &str = "json_get_text";
    pub const JSON_PATH: &str = "json_path";
    pub const JSON_PATH_TEXT: &str = "json_path_text";
    pub const JSON_HAS_KEY: &str = "json_has_key";
    pub const JSON_HAS_ANY_KEY: &str = "json_has_any_key";
    pub const JSON_HAS_ALL_KEYS: &str = "json_has_all_keys";
    pub const JSON_SET: &str = "json_set";
    pub const JSON_INSERT: &str = "json_insert";
    pub const JSON_REMOVE: &str = "json_remove";
    pub const JSON_REPLACE: &str = "json_replace";
    pub const JSON_MERGE_PATCH: &str = "json_merge_patch";
    pub const JSON_ARRAY: &str = "json_array";
    pub const JSON_OBJECT: &str = "json_object";
    pub const JSON_ARRAY_LENGTH: &str = "json_array_length";
    pub const JSON_KEYS: &str = "json_keys";
    pub const JSON_VALUES: &str = "json_values";
    pub const JSON_TYPEOF: &str = "json_typeof";

    // ── Array / Collection ───────────────────────────────────────────────
    pub const ARRAY_LENGTH: &str = "array_length";
    pub const ARRAY_POSITION: &str = "array_position";
    pub const ARRAY_APPEND: &str = "array_append";
    pub const ARRAY_PREPEND: &str = "array_prepend";
    pub const ARRAY_REMOVE: &str = "array_remove";
    pub const ARRAY_CAT: &str = "array_cat";
    pub const ARRAY_DISTINCT: &str = "array_distinct";
    pub const ARRAY_SORT: &str = "array_sort";
    pub const ARRAY_REVERSE: &str = "array_reverse";
    pub const ARRAY_SLICE: &str = "array_slice";
    pub const ARRAY_FLATTEN: &str = "array_flatten";
    pub const UNNEST: &str = "unnest";
    pub const ARRAY_TO_STRING: &str = "array_to_string";
    pub const STRING_TO_ARRAY: &str = "string_to_array";

    // ── Object / Map ─────────────────────────────────────────────────────
    pub const MAP_MERGE: &str = "map_merge";
    pub const MAP_GET: &str = "map_get";
    pub const MAP_KEYS: &str = "map_keys";
    pub const MAP_VALUES: &str = "map_values";
    pub const MAP_CONTAINS_KEY: &str = "map_contains_key";
    pub const MAP_REMOVE_KEY: &str = "map_remove_key";

    // ── Range operations ─────────────────────────────────────────────────
    pub const RANGE_CONTAINS: &str = "range_contains";
    pub const RANGE_CONTAINED_BY: &str = "range_contained_by";
    pub const RANGE_OVERLAP: &str = "range_overlap";
    pub const RANGE_LOWER: &str = "range_lower";
    pub const RANGE_UPPER: &str = "range_upper";
    pub const RANGE_IS_EMPTY: &str = "range_is_empty";

    // ── Window / Ranking ─────────────────────────────────────────────────
    pub const ROW_NUMBER: &str = "row_number";
    pub const RANK: &str = "rank";
    pub const DENSE_RANK: &str = "dense_rank";
    pub const NTILE: &str = "ntile";
    pub const LAG: &str = "lag";
    pub const LEAD: &str = "lead";
    pub const FIRST_VALUE: &str = "first_value";
    pub const LAST_VALUE: &str = "last_value";
    pub const NTH_VALUE: &str = "nth_value";
    pub const CUME_DIST: &str = "cume_dist";
    pub const PERCENT_RANK: &str = "percent_rank";

    // ── UUID ─────────────────────────────────────────────────────────────
    pub const GEN_RANDOM_UUID: &str = "gen_random_uuid";

    // ── Geo / Spatial ────────────────────────────────────────────────────
    pub const ST_CONTAINS: &str = "st_contains";
    pub const ST_INTERSECTS: &str = "st_intersects";
    pub const ST_WITHIN: &str = "st_within";
    pub const ST_AREA: &str = "st_area";
    pub const ST_LENGTH: &str = "st_length";
    pub const ST_DISTANCE: &str = "st_distance";
    pub const ST_BUFFER: &str = "st_buffer";
    pub const ST_CENTROID: &str = "st_centroid";
    pub const ST_AS_TEXT: &str = "st_as_text";
    pub const ST_GEOM_FROM_TEXT: &str = "st_geom_from_text";

    // ── Hashing / Encoding ───────────────────────────────────────────────
    pub const HASH: &str = "hash";
    pub const CRC32: &str = "crc32";
    pub const HEX_ENCODE: &str = "hex_encode";
    pub const HEX_DECODE: &str = "hex_decode";

    /// Returns `true` if this is a well-known "no-parens" SQL keyword
    /// (e.g. `CURRENT_DATE`).
    pub fn is_no_parens_keyword(&self) -> bool {
        matches!(
            self.as_str(),
            Self::CURRENT_DATE | Self::CURRENT_TIME | Self::CURRENT_TIMESTAMP
        )
    }
}

impl fmt::Display for FuncId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<&str> for FuncId {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_ref() == *other
    }
}

impl PartialEq<FuncId> for &str {
    fn eq(&self, other: &FuncId) -> bool {
        *self == other.0.as_ref()
    }
}

impl From<&str> for FuncId {
    fn from(s: &str) -> Self {
        Self(s.into())
    }
}

/// Backward-compatibility alias.
#[deprecated(note = "renamed to FuncId — use FuncId instead")]
pub type FuncName = FuncId;

// ---------------------------------------------------------------------------
// Generic function builders
// ---------------------------------------------------------------------------

/// Build a function-call expression from any name string.
pub fn func<'a>(name: &str, args: Vec<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: FuncId::new(name), args }
}

/// Internal helper to build a function-call expression from a well-known constant.
fn known<'a>(name: &str, args: Vec<Expr<'a>>) -> Expr<'a> {
    Expr::Func { name: FuncId::new(name), args }
}

// ---------------------------------------------------------------------------
// Aggregate functions
// ---------------------------------------------------------------------------

pub fn count<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::COUNT, vec![expr.into()])
}

pub fn count_star<'a>() -> Expr<'a> { Expr::CountStar }

pub fn sum<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::SUM, vec![expr.into()])
}

pub fn avg<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::AVG, vec![expr.into()])
}

pub fn min<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::MIN, vec![expr.into()])
}

pub fn max<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::MAX, vec![expr.into()])
}

// ---------------------------------------------------------------------------
// String functions
// ---------------------------------------------------------------------------

pub fn lower<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::LOWER, vec![expr.into()])
}

pub fn upper<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::UPPER, vec![expr.into()])
}

pub fn trim<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::TRIM, vec![expr.into()])
}

pub fn length<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::LENGTH, vec![expr.into()])
}

pub fn substr<'a>(expr: impl Into<Expr<'a>>, start: impl Into<Expr<'a>>, len: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::SUBSTR, vec![expr.into(), start.into(), len.into()])
}

pub fn concat_fn<'a>(args: Vec<Expr<'a>>) -> Expr<'a> {
    known(FuncId::CONCAT, args)
}

pub fn replace<'a>(expr: impl Into<Expr<'a>>, from: impl Into<Expr<'a>>, to: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::REPLACE, vec![expr.into(), from.into(), to.into()])
}

// ---------------------------------------------------------------------------
// Date/Time functions
// ---------------------------------------------------------------------------

pub fn now<'a>() -> Expr<'a> {
    known(FuncId::NOW, vec![])
}

pub fn current_date<'a>() -> Expr<'a> {
    known(FuncId::CURRENT_DATE, vec![])
}

pub fn current_timestamp<'a>() -> Expr<'a> {
    known(FuncId::CURRENT_TIMESTAMP, vec![])
}

// ---------------------------------------------------------------------------
// Null-handling functions
// ---------------------------------------------------------------------------

pub fn coalesce<'a>(args: Vec<Expr<'a>>) -> Expr<'a> {
    known(FuncId::COALESCE, args)
}

pub fn nullif<'a>(expr1: impl Into<Expr<'a>>, expr2: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::NULLIF, vec![expr1.into(), expr2.into()])
}

// ---------------------------------------------------------------------------
// Window functions
// ---------------------------------------------------------------------------

pub fn row_number<'a>() -> Expr<'a> {
    known(FuncId::ROW_NUMBER, vec![])
}

pub fn rank<'a>() -> Expr<'a> {
    known(FuncId::RANK, vec![])
}

pub fn dense_rank<'a>() -> Expr<'a> {
    known(FuncId::DENSE_RANK, vec![])
}

pub fn ntile<'a>(n: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::NTILE, vec![n.into()])
}

pub fn lag<'a>(expr: impl Into<Expr<'a>>, offset: Option<Expr<'a>>, default: Option<Expr<'a>>) -> Expr<'a> {
    let mut args = vec![expr.into()];
    if let Some(o) = offset { args.push(o); }
    if let Some(d) = default { args.push(d); }
    known(FuncId::LAG, args)
}

pub fn lead<'a>(expr: impl Into<Expr<'a>>, offset: Option<Expr<'a>>, default: Option<Expr<'a>>) -> Expr<'a> {
    let mut args = vec![expr.into()];
    if let Some(o) = offset { args.push(o); }
    if let Some(d) = default { args.push(d); }
    known(FuncId::LEAD, args)
}

pub fn first_value<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::FIRST_VALUE, vec![expr.into()])
}

pub fn last_value<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::LAST_VALUE, vec![expr.into()])
}

// ---------------------------------------------------------------------------
// Math / Numeric functions
// ---------------------------------------------------------------------------

pub fn abs<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::ABS, vec![expr.into()])
}

pub fn round<'a>(expr: impl Into<Expr<'a>>, precision: Option<Expr<'a>>) -> Expr<'a> {
    let mut args = vec![expr.into()];
    if let Some(p) = precision { args.push(p); }
    known(FuncId::ROUND, args)
}

pub fn sqrt<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::SQRT, vec![expr.into()])
}

pub fn cbrt<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::CBRT, vec![expr.into()])
}

pub fn factorial<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::FACTORIAL, vec![expr.into()])
}

pub fn greatest<'a>(args: Vec<Expr<'a>>) -> Expr<'a> {
    known(FuncId::GREATEST, args)
}

pub fn least<'a>(args: Vec<Expr<'a>>) -> Expr<'a> {
    known(FuncId::LEAST, args)
}

// ---------------------------------------------------------------------------
// JSON functions
// ---------------------------------------------------------------------------

pub fn json_get<'a>(doc: impl Into<Expr<'a>>, key: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::JSON_GET, vec![doc.into(), key.into()])
}

pub fn json_has_key<'a>(doc: impl Into<Expr<'a>>, key: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::JSON_HAS_KEY, vec![doc.into(), key.into()])
}

// ---------------------------------------------------------------------------
// Array functions
// ---------------------------------------------------------------------------

pub fn array_append<'a>(arr: impl Into<Expr<'a>>, elem: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::ARRAY_APPEND, vec![arr.into(), elem.into()])
}

pub fn array_prepend<'a>(elem: impl Into<Expr<'a>>, arr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::ARRAY_PREPEND, vec![elem.into(), arr.into()])
}

pub fn array_length<'a>(arr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::ARRAY_LENGTH, vec![arr.into()])
}

pub fn unnest<'a>(arr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::UNNEST, vec![arr.into()])
}

// ---------------------------------------------------------------------------
// Geo / Spatial functions
// ---------------------------------------------------------------------------

pub fn st_distance<'a>(a: impl Into<Expr<'a>>, b: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::ST_DISTANCE, vec![a.into(), b.into()])
}
