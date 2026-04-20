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
    pub const COUNT: &str = "COUNT";
    pub const COUNT_DISTINCT: &str = "COUNT_DISTINCT";
    pub const SUM: &str = "SUM";
    pub const AVG: &str = "AVG";
    pub const MIN: &str = "MIN";
    pub const MAX: &str = "MAX";
    pub const MEDIAN: &str = "MEDIAN";
    pub const STDDEV: &str = "STDDEV";
    pub const VARIANCE: &str = "VARIANCE";
    pub const ARRAY_AGG: &str = "ARRAY_AGG";
    pub const STRING_AGG: &str = "STRING_AGG";
    pub const JSON_AGG: &str = "JSON_AGG";
    pub const BOOL_AND: &str = "BOOL_AND";
    pub const BOOL_OR: &str = "BOOL_OR";
    pub const FIRST: &str = "FIRST";
    pub const LAST: &str = "LAST";

    // ── String ───────────────────────────────────────────────────────────
    pub const LOWER: &str = "LOWER";
    pub const UPPER: &str = "UPPER";
    pub const TRIM: &str = "TRIM";
    pub const LTRIM: &str = "LTRIM";
    pub const RTRIM: &str = "RTRIM";
    pub const LENGTH: &str = "LENGTH";
    pub const CHAR_LENGTH: &str = "CHAR_LENGTH";
    pub const OCTET_LENGTH: &str = "OCTET_LENGTH";
    pub const SUBSTR: &str = "SUBSTR";
    pub const LEFT: &str = "LEFT";
    pub const RIGHT: &str = "RIGHT";
    pub const CONCAT: &str = "CONCAT";
    pub const CONCAT_WS: &str = "CONCAT_WS";
    pub const REPLACE: &str = "REPLACE";
    pub const REVERSE: &str = "REVERSE";
    pub const REPEAT: &str = "REPEAT";
    pub const PAD_LEFT: &str = "PAD_LEFT";
    pub const PAD_RIGHT: &str = "PAD_RIGHT";
    pub const POSITION: &str = "POSITION";
    pub const INITCAP: &str = "INITCAP";
    pub const ASCII: &str = "ASCII";
    pub const CHR: &str = "CHR";
    pub const MD5: &str = "MD5";
    pub const SHA256: &str = "SHA256";
    pub const BASE64_ENCODE: &str = "BASE64_ENCODE";
    pub const BASE64_DECODE: &str = "BASE64_DECODE";
    pub const REGEX_REPLACE: &str = "REGEX_REPLACE";
    pub const REGEX_EXTRACT: &str = "REGEX_EXTRACT";
    pub const SPLIT: &str = "SPLIT";
    pub const SPLIT_PART: &str = "SPLIT_PART";
    pub const FORMAT: &str = "FORMAT";
    pub const STARTS_WITH: &str = "STARTS_WITH";
    pub const CONTAINS: &str = "CONTAINS";
    pub const TO_HEX: &str = "TO_HEX";

    // ── Numeric / Math ───────────────────────────────────────────────────
    pub const ABS: &str = "ABS";
    pub const CEIL: &str = "CEIL";
    pub const FLOOR: &str = "FLOOR";
    pub const ROUND: &str = "ROUND";
    pub const TRUNC: &str = "TRUNC";
    pub const SIGN: &str = "SIGN";
    pub const POWER: &str = "POWER";
    pub const SQRT: &str = "SQRT";
    pub const CBRT: &str = "CBRT";
    pub const EXP: &str = "EXP";
    pub const LN: &str = "LN";
    pub const LOG: &str = "LOG";
    pub const LOG2: &str = "LOG2";
    pub const LOG10: &str = "LOG10";
    pub const PI: &str = "PI";
    pub const DEGREES: &str = "DEGREES";
    pub const RADIANS: &str = "RADIANS";
    pub const SIN: &str = "SIN";
    pub const COS: &str = "COS";
    pub const TAN: &str = "TAN";
    pub const ASIN: &str = "ASIN";
    pub const ACOS: &str = "ACOS";
    pub const ATAN: &str = "ATAN";
    pub const ATAN2: &str = "ATAN2";
    pub const SINH: &str = "SINH";
    pub const COSH: &str = "COSH";
    pub const TANH: &str = "TANH";
    pub const FACTORIAL: &str = "FACTORIAL";
    pub const GCD: &str = "GCD";
    pub const LCM: &str = "LCM";
    pub const RANDOM: &str = "RANDOM";
    pub const GREATEST: &str = "GREATEST";
    pub const LEAST: &str = "LEAST";

    // ── Date / Time ──────────────────────────────────────────────────────
    pub const NOW: &str = "NOW";
    pub const CURRENT_DATE: &str = "CURRENT_DATE";
    pub const CURRENT_TIME: &str = "CURRENT_TIME";
    pub const CURRENT_TIMESTAMP: &str = "CURRENT_TIMESTAMP";
    pub const DATE_PART: &str = "DATE_PART";
    pub const DATE_TRUNC: &str = "DATE_TRUNC";
    pub const EXTRACT: &str = "EXTRACT";
    pub const DATE_ADD: &str = "DATE_ADD";
    pub const DATE_SUB: &str = "DATE_SUB";
    pub const DATE_DIFF: &str = "DATE_DIFF";
    pub const AGE: &str = "AGE";
    pub const TO_DATE: &str = "TO_DATE";
    pub const TO_TIMESTAMP: &str = "TO_TIMESTAMP";
    pub const YEAR: &str = "YEAR";
    pub const MONTH: &str = "MONTH";
    pub const DAY: &str = "DAY";
    pub const HOUR: &str = "HOUR";
    pub const MINUTE: &str = "MINUTE";
    pub const SECOND: &str = "SECOND";
    pub const DAY_OF_WEEK: &str = "DAY_OF_WEEK";
    pub const DAY_OF_YEAR: &str = "DAY_OF_YEAR";
    pub const WEEK_OF_YEAR: &str = "WEEK_OF_YEAR";
    pub const QUARTER: &str = "QUARTER";
    pub const MAKE_DATE: &str = "MAKE_DATE";
    pub const MAKE_TIME: &str = "MAKE_TIME";
    pub const MAKE_TIMESTAMP: &str = "MAKE_TIMESTAMP";
    pub const EPOCH_TO_TIMESTAMP: &str = "EPOCH_TO_TIMESTAMP";
    pub const TIMESTAMP_TO_EPOCH: &str = "TIMESTAMP_TO_EPOCH";

    // ── Null-handling ────────────────────────────────────────────────────
    pub const COALESCE: &str = "COALESCE";
    pub const NULLIF: &str = "NULLIF";
    pub const IFNULL: &str = "IFNULL";

    // ── Type conversion ──────────────────────────────────────────────────
    pub const TYPEOF: &str = "TYPEOF";
    pub const TO_TEXT: &str = "TO_TEXT";
    pub const TO_INT: &str = "TO_INT";
    pub const TO_FLOAT: &str = "TO_FLOAT";
    pub const TO_BOOL: &str = "TO_BOOL";

    // ── JSON / Document ──────────────────────────────────────────────────
    pub const JSON_GET: &str = "JSON_GET";
    pub const JSON_GET_TEXT: &str = "JSON_GET_TEXT";
    pub const JSON_PATH: &str = "JSON_PATH";
    pub const JSON_PATH_TEXT: &str = "JSON_PATH_TEXT";
    pub const JSON_HAS_KEY: &str = "JSON_HAS_KEY";
    pub const JSON_HAS_ANY_KEY: &str = "JSON_HAS_ANY_KEY";
    pub const JSON_HAS_ALL_KEYS: &str = "JSON_HAS_ALL_KEYS";
    pub const JSON_SET: &str = "JSON_SET";
    pub const JSON_INSERT: &str = "JSON_INSERT";
    pub const JSON_REMOVE: &str = "JSON_REMOVE";
    pub const JSON_REPLACE: &str = "JSON_REPLACE";
    pub const JSON_MERGE_PATCH: &str = "JSON_MERGE_PATCH";
    pub const JSON_ARRAY: &str = "JSON_ARRAY";
    pub const JSON_OBJECT: &str = "JSON_OBJECT";
    pub const JSON_ARRAY_LENGTH: &str = "JSON_ARRAY_LENGTH";
    pub const JSON_KEYS: &str = "JSON_KEYS";
    pub const JSON_VALUES: &str = "JSON_VALUES";
    pub const JSON_TYPEOF: &str = "JSON_TYPEOF";

    // ── Array / Collection ───────────────────────────────────────────────
    pub const ARRAY_LENGTH: &str = "ARRAY_LENGTH";
    pub const ARRAY_POSITION: &str = "ARRAY_POSITION";
    pub const ARRAY_APPEND: &str = "ARRAY_APPEND";
    pub const ARRAY_PREPEND: &str = "ARRAY_PREPEND";
    pub const ARRAY_REMOVE: &str = "ARRAY_REMOVE";
    pub const ARRAY_CAT: &str = "ARRAY_CAT";
    pub const ARRAY_DISTINCT: &str = "ARRAY_DISTINCT";
    pub const ARRAY_SORT: &str = "ARRAY_SORT";
    pub const ARRAY_REVERSE: &str = "ARRAY_REVERSE";
    pub const ARRAY_SLICE: &str = "ARRAY_SLICE";
    pub const ARRAY_FLATTEN: &str = "ARRAY_FLATTEN";
    pub const UNNEST: &str = "UNNEST";
    pub const ARRAY_TO_STRING: &str = "ARRAY_TO_STRING";
    pub const STRING_TO_ARRAY: &str = "STRING_TO_ARRAY";

    // ── Object / Map ─────────────────────────────────────────────────────
    pub const MAP_MERGE: &str = "MAP_MERGE";
    pub const MAP_GET: &str = "MAP_GET";
    pub const MAP_KEYS: &str = "MAP_KEYS";
    pub const MAP_VALUES: &str = "MAP_VALUES";
    pub const MAP_CONTAINS_KEY: &str = "MAP_CONTAINS_KEY";
    pub const MAP_REMOVE_KEY: &str = "MAP_REMOVE_KEY";

    // ── Range operations ─────────────────────────────────────────────────
    pub const RANGE_CONTAINS: &str = "RANGE_CONTAINS";
    pub const RANGE_CONTAINED_BY: &str = "RANGE_CONTAINED_BY";
    pub const RANGE_OVERLAP: &str = "RANGE_OVERLAP";
    pub const RANGE_LOWER: &str = "RANGE_LOWER";
    pub const RANGE_UPPER: &str = "RANGE_UPPER";
    pub const RANGE_IS_EMPTY: &str = "RANGE_IS_EMPTY";

    // ── Window / Ranking ─────────────────────────────────────────────────
    pub const ROW_NUMBER: &str = "ROW_NUMBER";
    pub const RANK: &str = "RANK";
    pub const DENSE_RANK: &str = "DENSE_RANK";
    pub const NTILE: &str = "NTILE";
    pub const LAG: &str = "LAG";
    pub const LEAD: &str = "LEAD";
    pub const FIRST_VALUE: &str = "FIRST_VALUE";
    pub const LAST_VALUE: &str = "LAST_VALUE";
    pub const NTH_VALUE: &str = "NTH_VALUE";
    pub const CUME_DIST: &str = "CUME_DIST";
    pub const PERCENT_RANK: &str = "PERCENT_RANK";

    // ── UUID ─────────────────────────────────────────────────────────────
    pub const GEN_RANDOM_UUID: &str = "GEN_RANDOM_UUID";

    // ── Geo / Spatial ────────────────────────────────────────────────────
    pub const ST_CONTAINS: &str = "ST_CONTAINS";
    pub const ST_INTERSECTS: &str = "ST_INTERSECTS";
    pub const ST_WITHIN: &str = "ST_WITHIN";
    pub const ST_AREA: &str = "ST_AREA";
    pub const ST_LENGTH: &str = "ST_LENGTH";
    pub const ST_DISTANCE: &str = "ST_DISTANCE";
    pub const ST_BUFFER: &str = "ST_BUFFER";
    pub const ST_CENTROID: &str = "ST_CENTROID";
    pub const ST_AS_TEXT: &str = "ST_AS_TEXT";
    pub const ST_GEOM_FROM_TEXT: &str = "ST_GEOM_FROM_TEXT";

    // ── Hashing / Encoding ───────────────────────────────────────────────
    pub const HASH: &str = "HASH";
    pub const CRC32: &str = "CRC32";
    pub const HEX_ENCODE: &str = "HEX_ENCODE";
    pub const HEX_DECODE: &str = "HEX_DECODE";

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
    Expr::Func {
        name: FuncId::new(name),
        args,
    }
}

/// Internal helper to build a function-call expression from a well-known constant.
fn known<'a>(name: &str, args: Vec<Expr<'a>>) -> Expr<'a> {
    Expr::Func {
        name: FuncId::new(name),
        args,
    }
}

// ---------------------------------------------------------------------------
// Aggregate functions
// ---------------------------------------------------------------------------

pub fn count<'a>(expr: impl Into<Expr<'a>>) -> Expr<'a> {
    known(FuncId::COUNT, vec![expr.into()])
}

pub fn count_star<'a>() -> Expr<'a> {
    Expr::CountStar
}

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

pub fn substr<'a>(
    expr: impl Into<Expr<'a>>,
    start: impl Into<Expr<'a>>,
    len: impl Into<Expr<'a>>,
) -> Expr<'a> {
    known(FuncId::SUBSTR, vec![expr.into(), start.into(), len.into()])
}

pub fn concat_fn<'a>(args: Vec<Expr<'a>>) -> Expr<'a> {
    known(FuncId::CONCAT, args)
}

pub fn replace<'a>(
    expr: impl Into<Expr<'a>>,
    from: impl Into<Expr<'a>>,
    to: impl Into<Expr<'a>>,
) -> Expr<'a> {
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
    known(FuncId::LAG, args)
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
    if let Some(p) = precision {
        args.push(p);
    }
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
