//! Registry of all well-known DOL functions as zero-sized structs.
//!
//! Each struct implements [`DolFunc`] via the [`define_func!`] macro,
//! providing a canonical name, arity constraint, and function kind.
//!
//! Backends can extend any struct with additional traits:
//!
//! ```ignore
//! pub trait PostgresFunc: DolFunc {
//!     fn pg_name() -> &'static str { Self::NAME }
//! }
//! impl PostgresFunc for PadLeft {
//!     fn pg_name() -> &'static str { "LPAD" }
//! }
//! ```

use super::func_def::{Arity, FuncKind};
use crate::define_func;

// ═══════════════════════════════════════════════════════════════════════════
// Aggregate functions
// ═══════════════════════════════════════════════════════════════════════════

define_func!(Count, "COUNT", Arity::Exact(1), FuncKind::Aggregate);
define_func!(CountDistinct, "COUNT_DISTINCT", Arity::Exact(1), FuncKind::Aggregate);
define_func!(Sum, "SUM", Arity::Exact(1), FuncKind::Aggregate);
define_func!(Avg, "AVG", Arity::Exact(1), FuncKind::Aggregate);
define_func!(Min, "MIN", Arity::Exact(1), FuncKind::Aggregate);
define_func!(Max, "MAX", Arity::Exact(1), FuncKind::Aggregate);
define_func!(Median, "MEDIAN", Arity::Exact(1), FuncKind::Aggregate);
define_func!(Stddev, "STDDEV", Arity::Exact(1), FuncKind::Aggregate);
define_func!(Variance, "VARIANCE", Arity::Exact(1), FuncKind::Aggregate);
define_func!(ArrayAgg, "ARRAY_AGG", Arity::Exact(1), FuncKind::Aggregate);
define_func!(StringAgg, "STRING_AGG", Arity::Exact(2), FuncKind::Aggregate);
define_func!(JsonAgg, "JSON_AGG", Arity::Exact(1), FuncKind::Aggregate);
define_func!(BoolAnd, "BOOL_AND", Arity::Exact(1), FuncKind::Aggregate);
define_func!(BoolOr, "BOOL_OR", Arity::Exact(1), FuncKind::Aggregate);
define_func!(First, "FIRST", Arity::Exact(1), FuncKind::Aggregate);
define_func!(Last, "LAST", Arity::Exact(1), FuncKind::Aggregate);

// ═══════════════════════════════════════════════════════════════════════════
// String functions
// ═══════════════════════════════════════════════════════════════════════════

define_func!(Lower, "LOWER", Arity::Exact(1), FuncKind::Scalar);
define_func!(Upper, "UPPER", Arity::Exact(1), FuncKind::Scalar);
define_func!(Trim, "TRIM", Arity::Exact(1), FuncKind::Scalar);
define_func!(Ltrim, "LTRIM", Arity::Range(1, 2), FuncKind::Scalar);
define_func!(Rtrim, "RTRIM", Arity::Range(1, 2), FuncKind::Scalar);
define_func!(Length, "LENGTH", Arity::Exact(1), FuncKind::Scalar);
define_func!(CharLength, "CHAR_LENGTH", Arity::Exact(1), FuncKind::Scalar);
define_func!(OctetLength, "OCTET_LENGTH", Arity::Exact(1), FuncKind::Scalar);
define_func!(Substr, "SUBSTR", Arity::Range(2, 3), FuncKind::Scalar);
define_func!(Left, "LEFT", Arity::Exact(2), FuncKind::Scalar);
define_func!(Right, "RIGHT", Arity::Exact(2), FuncKind::Scalar);
define_func!(Concat, "CONCAT", Arity::AtLeast(1), FuncKind::Scalar);
define_func!(ConcatWs, "CONCAT_WS", Arity::AtLeast(2), FuncKind::Scalar);
define_func!(Replace, "REPLACE", Arity::Exact(3), FuncKind::Scalar);
define_func!(Reverse, "REVERSE", Arity::Exact(1), FuncKind::Scalar);
define_func!(Repeat, "REPEAT", Arity::Exact(2), FuncKind::Scalar);
define_func!(PadLeft, "PAD_LEFT", Arity::Range(2, 3), FuncKind::Scalar);
define_func!(PadRight, "PAD_RIGHT", Arity::Range(2, 3), FuncKind::Scalar);
define_func!(Position, "POSITION", Arity::Exact(2), FuncKind::Scalar);
define_func!(Initcap, "INITCAP", Arity::Exact(1), FuncKind::Scalar);
define_func!(Ascii, "ASCII", Arity::Exact(1), FuncKind::Scalar);
define_func!(Chr, "CHR", Arity::Exact(1), FuncKind::Scalar);
define_func!(Md5, "MD5", Arity::Exact(1), FuncKind::Scalar);
define_func!(Sha256, "SHA256", Arity::Exact(1), FuncKind::Scalar);
define_func!(Base64Encode, "BASE64_ENCODE", Arity::Exact(1), FuncKind::Scalar);
define_func!(Base64Decode, "BASE64_DECODE", Arity::Exact(1), FuncKind::Scalar);
define_func!(RegexReplace, "REGEX_REPLACE", Arity::Range(3, 4), FuncKind::Scalar);
define_func!(RegexExtract, "REGEX_EXTRACT", Arity::Exact(2), FuncKind::Scalar);
define_func!(Split, "SPLIT", Arity::Exact(2), FuncKind::Scalar);
define_func!(SplitPart, "SPLIT_PART", Arity::Exact(3), FuncKind::Scalar);
define_func!(Format, "FORMAT", Arity::AtLeast(1), FuncKind::Scalar);
define_func!(StartsWith, "STARTS_WITH", Arity::Exact(2), FuncKind::Scalar);
define_func!(Contains, "CONTAINS", Arity::Exact(2), FuncKind::Scalar);
define_func!(ToHex, "TO_HEX", Arity::Exact(1), FuncKind::Scalar);

// ═══════════════════════════════════════════════════════════════════════════
// Numeric / Math functions
// ═══════════════════════════════════════════════════════════════════════════

define_func!(Abs, "ABS", Arity::Exact(1), FuncKind::Scalar);
define_func!(Ceil, "CEIL", Arity::Exact(1), FuncKind::Scalar);
define_func!(Floor, "FLOOR", Arity::Exact(1), FuncKind::Scalar);
define_func!(Round, "ROUND", Arity::Range(1, 2), FuncKind::Scalar);
define_func!(Trunc, "TRUNC", Arity::Range(1, 2), FuncKind::Scalar);
define_func!(Sign, "SIGN", Arity::Exact(1), FuncKind::Scalar);
define_func!(Power, "POWER", Arity::Exact(2), FuncKind::Scalar);
define_func!(Sqrt, "SQRT", Arity::Exact(1), FuncKind::Scalar);
define_func!(Cbrt, "CBRT", Arity::Exact(1), FuncKind::Scalar);
define_func!(Exp, "EXP", Arity::Exact(1), FuncKind::Scalar);
define_func!(Ln, "LN", Arity::Exact(1), FuncKind::Scalar);
define_func!(Log, "LOG", Arity::Range(1, 2), FuncKind::Scalar);
define_func!(Log2, "LOG2", Arity::Exact(1), FuncKind::Scalar);
define_func!(Log10, "LOG10", Arity::Exact(1), FuncKind::Scalar);
define_func!(Pi, "PI", Arity::Exact(0), FuncKind::Scalar);
define_func!(Degrees, "DEGREES", Arity::Exact(1), FuncKind::Scalar);
define_func!(Radians, "RADIANS", Arity::Exact(1), FuncKind::Scalar);
define_func!(Sin, "SIN", Arity::Exact(1), FuncKind::Scalar);
define_func!(Cos, "COS", Arity::Exact(1), FuncKind::Scalar);
define_func!(Tan, "TAN", Arity::Exact(1), FuncKind::Scalar);
define_func!(Asin, "ASIN", Arity::Exact(1), FuncKind::Scalar);
define_func!(Acos, "ACOS", Arity::Exact(1), FuncKind::Scalar);
define_func!(Atan, "ATAN", Arity::Exact(1), FuncKind::Scalar);
define_func!(Atan2, "ATAN2", Arity::Exact(2), FuncKind::Scalar);
define_func!(Sinh, "SINH", Arity::Exact(1), FuncKind::Scalar);
define_func!(Cosh, "COSH", Arity::Exact(1), FuncKind::Scalar);
define_func!(Tanh, "TANH", Arity::Exact(1), FuncKind::Scalar);
define_func!(Factorial, "FACTORIAL", Arity::Exact(1), FuncKind::Scalar);
define_func!(Gcd, "GCD", Arity::Exact(2), FuncKind::Scalar);
define_func!(Lcm, "LCM", Arity::Exact(2), FuncKind::Scalar);
define_func!(Random, "RANDOM", Arity::Exact(0), FuncKind::Scalar);
define_func!(Greatest, "GREATEST", Arity::AtLeast(1), FuncKind::Scalar);
define_func!(Least, "LEAST", Arity::AtLeast(1), FuncKind::Scalar);

// ═══════════════════════════════════════════════════════════════════════════
// Date / Time functions
// ═══════════════════════════════════════════════════════════════════════════

define_func!(Now, "NOW", Arity::Exact(0), FuncKind::Scalar);
define_func!(CurrentDate, "CURRENT_DATE", Arity::Exact(0), FuncKind::Scalar);
define_func!(CurrentTime, "CURRENT_TIME", Arity::Exact(0), FuncKind::Scalar);
define_func!(CurrentTimestamp, "CURRENT_TIMESTAMP", Arity::Exact(0), FuncKind::Scalar);
define_func!(DatePart, "DATE_PART", Arity::Exact(2), FuncKind::Scalar);
define_func!(DateTrunc, "DATE_TRUNC", Arity::Exact(2), FuncKind::Scalar);
define_func!(Extract, "EXTRACT", Arity::Exact(2), FuncKind::Scalar);
define_func!(DateAdd, "DATE_ADD", Arity::Range(2, 3), FuncKind::Scalar);
define_func!(DateSub, "DATE_SUB", Arity::Range(2, 3), FuncKind::Scalar);
define_func!(DateDiff, "DATE_DIFF", Arity::Range(2, 3), FuncKind::Scalar);
define_func!(Age, "AGE", Arity::Range(1, 2), FuncKind::Scalar);
define_func!(ToDate, "TO_DATE", Arity::Range(1, 2), FuncKind::Scalar);
define_func!(ToTimestamp, "TO_TIMESTAMP", Arity::Range(1, 2), FuncKind::Scalar);
define_func!(Year, "YEAR", Arity::Exact(1), FuncKind::Scalar);
define_func!(Month, "MONTH", Arity::Exact(1), FuncKind::Scalar);
define_func!(Day, "DAY", Arity::Exact(1), FuncKind::Scalar);
define_func!(Hour, "HOUR", Arity::Exact(1), FuncKind::Scalar);
define_func!(Minute, "MINUTE", Arity::Exact(1), FuncKind::Scalar);
define_func!(Second, "SECOND", Arity::Exact(1), FuncKind::Scalar);
define_func!(DayOfWeek, "DAY_OF_WEEK", Arity::Exact(1), FuncKind::Scalar);
define_func!(DayOfYear, "DAY_OF_YEAR", Arity::Exact(1), FuncKind::Scalar);
define_func!(WeekOfYear, "WEEK_OF_YEAR", Arity::Exact(1), FuncKind::Scalar);
define_func!(Quarter, "QUARTER", Arity::Exact(1), FuncKind::Scalar);
define_func!(MakeDate, "MAKE_DATE", Arity::Exact(3), FuncKind::Scalar);
define_func!(MakeTime, "MAKE_TIME", Arity::Exact(3), FuncKind::Scalar);
define_func!(MakeTimestamp, "MAKE_TIMESTAMP", Arity::Exact(6), FuncKind::Scalar);
define_func!(EpochToTimestamp, "EPOCH_TO_TIMESTAMP", Arity::Exact(1), FuncKind::Scalar);
define_func!(TimestampToEpoch, "TIMESTAMP_TO_EPOCH", Arity::Exact(1), FuncKind::Scalar);

// ═══════════════════════════════════════════════════════════════════════════
// Null-handling functions
// ═══════════════════════════════════════════════════════════════════════════

define_func!(Coalesce, "COALESCE", Arity::AtLeast(1), FuncKind::Scalar);
define_func!(Nullif, "NULLIF", Arity::Exact(2), FuncKind::Scalar);
define_func!(Ifnull, "IFNULL", Arity::Exact(2), FuncKind::Scalar);

// ═══════════════════════════════════════════════════════════════════════════
// Type conversion functions
// ═══════════════════════════════════════════════════════════════════════════

define_func!(Typeof, "TYPEOF", Arity::Exact(1), FuncKind::Scalar);
define_func!(ToText, "TO_TEXT", Arity::Exact(1), FuncKind::Scalar);
define_func!(ToInt, "TO_INT", Arity::Exact(1), FuncKind::Scalar);
define_func!(ToFloat, "TO_FLOAT", Arity::Exact(1), FuncKind::Scalar);
define_func!(ToBool, "TO_BOOL", Arity::Exact(1), FuncKind::Scalar);

// ═══════════════════════════════════════════════════════════════════════════
// JSON / Document functions
// ═══════════════════════════════════════════════════════════════════════════

define_func!(JsonGet, "JSON_GET", Arity::Exact(2), FuncKind::Scalar);
define_func!(JsonGetText, "JSON_GET_TEXT", Arity::Exact(2), FuncKind::Scalar);
define_func!(JsonPath, "JSON_PATH", Arity::AtLeast(2), FuncKind::Scalar);
define_func!(JsonPathText, "JSON_PATH_TEXT", Arity::AtLeast(2), FuncKind::Scalar);
define_func!(JsonHasKey, "JSON_HAS_KEY", Arity::Exact(2), FuncKind::Scalar);
define_func!(JsonHasAnyKey, "JSON_HAS_ANY_KEY", Arity::Exact(2), FuncKind::Scalar);
define_func!(JsonHasAllKeys, "JSON_HAS_ALL_KEYS", Arity::Exact(2), FuncKind::Scalar);
define_func!(JsonSet, "JSON_SET", Arity::Exact(3), FuncKind::Scalar);
define_func!(JsonInsert, "JSON_INSERT", Arity::Exact(3), FuncKind::Scalar);
define_func!(JsonRemove, "JSON_REMOVE", Arity::Exact(2), FuncKind::Scalar);
define_func!(JsonReplace, "JSON_REPLACE", Arity::Exact(3), FuncKind::Scalar);
define_func!(JsonMergePatch, "JSON_MERGE_PATCH", Arity::Exact(2), FuncKind::Scalar);
define_func!(JsonArray, "JSON_ARRAY", Arity::Any, FuncKind::Scalar);
define_func!(JsonObject, "JSON_OBJECT", Arity::Any, FuncKind::Scalar);
define_func!(JsonArrayLength, "JSON_ARRAY_LENGTH", Arity::Exact(1), FuncKind::Scalar);
define_func!(JsonKeys, "JSON_KEYS", Arity::Exact(1), FuncKind::Scalar);
define_func!(JsonValues, "JSON_VALUES", Arity::Exact(1), FuncKind::Scalar);
define_func!(JsonTypeof, "JSON_TYPEOF", Arity::Exact(1), FuncKind::Scalar);

// ═══════════════════════════════════════════════════════════════════════════
// Array / Collection functions
// ═══════════════════════════════════════════════════════════════════════════

define_func!(ArrayLength, "ARRAY_LENGTH", Arity::Range(1, 2), FuncKind::Scalar);
define_func!(ArrayPosition, "ARRAY_POSITION", Arity::Exact(2), FuncKind::Scalar);
define_func!(ArrayAppend, "ARRAY_APPEND", Arity::Exact(2), FuncKind::Scalar);
define_func!(ArrayPrepend, "ARRAY_PREPEND", Arity::Exact(2), FuncKind::Scalar);
define_func!(ArrayRemove, "ARRAY_REMOVE", Arity::Exact(2), FuncKind::Scalar);
define_func!(ArrayCat, "ARRAY_CAT", Arity::Exact(2), FuncKind::Scalar);
define_func!(ArrayDistinct, "ARRAY_DISTINCT", Arity::Exact(1), FuncKind::Scalar);
define_func!(ArraySort, "ARRAY_SORT", Arity::Exact(1), FuncKind::Scalar);
define_func!(ArrayReverse, "ARRAY_REVERSE", Arity::Exact(1), FuncKind::Scalar);
define_func!(ArraySlice, "ARRAY_SLICE", Arity::Exact(3), FuncKind::Scalar);
define_func!(ArrayFlatten, "ARRAY_FLATTEN", Arity::Exact(1), FuncKind::Scalar);
define_func!(Unnest, "UNNEST", Arity::Exact(1), FuncKind::Scalar);
define_func!(ArrayToString, "ARRAY_TO_STRING", Arity::Range(2, 3), FuncKind::Scalar);
define_func!(StringToArray, "STRING_TO_ARRAY", Arity::Range(2, 3), FuncKind::Scalar);

// ═══════════════════════════════════════════════════════════════════════════
// Object / Map functions
// ═══════════════════════════════════════════════════════════════════════════

define_func!(MapMerge, "MAP_MERGE", Arity::Exact(2), FuncKind::Scalar);
define_func!(MapGet, "MAP_GET", Arity::Exact(2), FuncKind::Scalar);
define_func!(MapKeys, "MAP_KEYS", Arity::Exact(1), FuncKind::Scalar);
define_func!(MapValues, "MAP_VALUES", Arity::Exact(1), FuncKind::Scalar);
define_func!(MapContainsKey, "MAP_CONTAINS_KEY", Arity::Exact(2), FuncKind::Scalar);
define_func!(MapRemoveKey, "MAP_REMOVE_KEY", Arity::Exact(2), FuncKind::Scalar);

// ═══════════════════════════════════════════════════════════════════════════
// Range operations
// ═══════════════════════════════════════════════════════════════════════════

define_func!(RangeContains, "RANGE_CONTAINS", Arity::Exact(2), FuncKind::Scalar);
define_func!(RangeContainedBy, "RANGE_CONTAINED_BY", Arity::Exact(2), FuncKind::Scalar);
define_func!(RangeOverlap, "RANGE_OVERLAP", Arity::Exact(2), FuncKind::Scalar);
define_func!(RangeLower, "RANGE_LOWER", Arity::Exact(1), FuncKind::Scalar);
define_func!(RangeUpper, "RANGE_UPPER", Arity::Exact(1), FuncKind::Scalar);
define_func!(RangeIsEmpty, "RANGE_IS_EMPTY", Arity::Exact(1), FuncKind::Scalar);

// ═══════════════════════════════════════════════════════════════════════════
// Window / Ranking functions
// ═══════════════════════════════════════════════════════════════════════════

define_func!(RowNumber, "ROW_NUMBER", Arity::Exact(0), FuncKind::Window);
define_func!(Rank, "RANK", Arity::Exact(0), FuncKind::Window);
define_func!(DenseRank, "DENSE_RANK", Arity::Exact(0), FuncKind::Window);
define_func!(Ntile, "NTILE", Arity::Exact(1), FuncKind::Window);
define_func!(Lag, "LAG", Arity::Range(1, 3), FuncKind::Window);
define_func!(Lead, "LEAD", Arity::Range(1, 3), FuncKind::Window);
define_func!(FirstValue, "FIRST_VALUE", Arity::Exact(1), FuncKind::Window);
define_func!(LastValue, "LAST_VALUE", Arity::Exact(1), FuncKind::Window);
define_func!(NthValue, "NTH_VALUE", Arity::Exact(2), FuncKind::Window);
define_func!(CumeDist, "CUME_DIST", Arity::Exact(0), FuncKind::Window);
define_func!(PercentRank, "PERCENT_RANK", Arity::Exact(0), FuncKind::Window);

// ═══════════════════════════════════════════════════════════════════════════
// UUID functions
// ═══════════════════════════════════════════════════════════════════════════

define_func!(GenRandomUuid, "GEN_RANDOM_UUID", Arity::Exact(0), FuncKind::Scalar);

// ═══════════════════════════════════════════════════════════════════════════
// Geo / Spatial functions
// ═══════════════════════════════════════════════════════════════════════════

define_func!(StContains, "ST_CONTAINS", Arity::Exact(2), FuncKind::Scalar);
define_func!(StIntersects, "ST_INTERSECTS", Arity::Exact(2), FuncKind::Scalar);
define_func!(StWithin, "ST_WITHIN", Arity::Exact(2), FuncKind::Scalar);
define_func!(StArea, "ST_AREA", Arity::Exact(1), FuncKind::Scalar);
define_func!(StLength, "ST_LENGTH", Arity::Exact(1), FuncKind::Scalar);
define_func!(StDistance, "ST_DISTANCE", Arity::Exact(2), FuncKind::Scalar);
define_func!(StBuffer, "ST_BUFFER", Arity::Exact(2), FuncKind::Scalar);
define_func!(StCentroid, "ST_CENTROID", Arity::Exact(1), FuncKind::Scalar);
define_func!(StAsText, "ST_AS_TEXT", Arity::Exact(1), FuncKind::Scalar);
define_func!(StGeomFromText, "ST_GEOM_FROM_TEXT", Arity::Range(1, 2), FuncKind::Scalar);

// ═══════════════════════════════════════════════════════════════════════════
// Hashing / Encoding functions
// ═══════════════════════════════════════════════════════════════════════════

define_func!(Hash, "HASH", Arity::Exact(1), FuncKind::Scalar);
define_func!(Crc32, "CRC32", Arity::Exact(1), FuncKind::Scalar);
define_func!(HexEncode, "HEX_ENCODE", Arity::Exact(1), FuncKind::Scalar);
define_func!(HexDecode, "HEX_DECODE", Arity::Exact(1), FuncKind::Scalar);
