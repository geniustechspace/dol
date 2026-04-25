//! Function metadata and traits for well-known DOL functions.

use core::fmt;
use std::borrow::Cow;

use super::super::compact_name::CompactName;

/// Describes the expected argument count for a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Arity {
    /// Exactly `n` arguments required.
    Exact(u8),
    /// At least `n` arguments required (variadic).
    AtLeast(u8),
    /// Between `lo` and `hi` arguments (inclusive).
    Range(u8, u8),
    /// Any number of arguments (no validation).
    Any,
}

/// Error from arity validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArityError {
    /// The function name that failed validation.
    pub func_name: String,
    /// The arity constraint.
    pub expected: Arity,
    /// The actual number of arguments provided.
    pub actual: usize,
}

impl fmt::Display for ArityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "function '{}': expected {} argument(s), got {}",
            self.func_name,
            match self.expected {
                Arity::Exact(n) => format!("exactly {n}"),
                Arity::AtLeast(n) => format!("at least {n}"),
                Arity::Range(lo, hi) => format!("{lo}..={hi}"),
                Arity::Any => "any number of".to_string(),
            },
            self.actual,
        )
    }
}

/// Classification of a DOL function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FuncKind {
    /// A scalar function (e.g. `LOWER`, `ABS`, `ROUND`).
    Scalar,
    /// An aggregate function (e.g. `COUNT`, `SUM`, `AVG`).
    Aggregate,
    /// A window/ranking function (e.g. `ROW_NUMBER`, `RANK`).
    Window,
}

/// A rich function definition: name + arity + kind.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FuncDef {
    name: CompactName,
    arity: Arity,
    kind: FuncKind,
}

impl FuncDef {
    /// Create a new function definition (used by the `DolFunc` trait).
    pub const fn new_static(name: &'static str, arity: Arity, kind: FuncKind) -> Self {
        Self {
            name: CompactName::Static(name),
            arity,
            kind,
        }
    }

    /// Create a custom function definition.
    ///
    /// Custom functions default to `Arity::Any` and `FuncKind::Scalar`.
    pub fn custom(name: impl Into<Box<str>>) -> Self {
        Self {
            name: CompactName::Owned(name.into()),
            arity: Arity::Any,
            kind: FuncKind::Scalar,
        }
    }

    /// Create a custom function with explicit arity and kind.
    pub fn custom_with(name: impl Into<Box<str>>, arity: Arity, kind: FuncKind) -> Self {
        Self {
            name: CompactName::Owned(name.into()),
            arity,
            kind,
        }
    }

    /// Return the function name as a string.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Return the arity constraint.
    pub fn arity(&self) -> Arity {
        self.arity
    }

    /// Return the function kind.
    pub fn kind(&self) -> FuncKind {
        self.kind
    }

    /// Validate argument count against the arity constraint.
    pub fn validate_arity(&self, arg_count: usize) -> Result<(), ArityError> {
        let ok = match self.arity {
            Arity::Exact(n) => arg_count == n as usize,
            Arity::AtLeast(n) => arg_count >= n as usize,
            Arity::Range(lo, hi) => arg_count >= lo as usize && arg_count <= hi as usize,
            Arity::Any => true,
        };
        if ok {
            Ok(())
        } else {
            Err(ArityError {
                func_name: self.name().to_string(),
                expected: self.arity,
                actual: arg_count,
            })
        }
    }

    /// Returns `true` if this is a well-known no-parens SQL keyword
    /// (e.g. `CURRENT_DATE`).
    pub fn is_no_parens_keyword(&self) -> bool {
        matches!(
            self.name(),
            "CURRENT_DATE" | "CURRENT_TIME" | "CURRENT_TIMESTAMP"
        )
    }

    // Aggregate
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

    // String
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

    // Numeric / Math
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

    // Date / Time
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

    // Null-handling
    pub const COALESCE: &str = "COALESCE";
    pub const NULLIF: &str = "NULLIF";
    pub const IFNULL: &str = "IFNULL";

    // Type conversion
    pub const TYPEOF: &str = "TYPEOF";
    pub const TO_TEXT: &str = "TO_TEXT";
    pub const TO_INT: &str = "TO_INT";
    pub const TO_FLOAT: &str = "TO_FLOAT";
    pub const TO_BOOL: &str = "TO_BOOL";

    // JSON / Document
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

    // Array / Collection
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

    // Object / Map
    pub const MAP_MERGE: &str = "MAP_MERGE";
    pub const MAP_GET: &str = "MAP_GET";
    pub const MAP_KEYS: &str = "MAP_KEYS";
    pub const MAP_VALUES: &str = "MAP_VALUES";
    pub const MAP_CONTAINS_KEY: &str = "MAP_CONTAINS_KEY";
    pub const MAP_REMOVE_KEY: &str = "MAP_REMOVE_KEY";

    // Range operations
    pub const RANGE_CONTAINS: &str = "RANGE_CONTAINS";
    pub const RANGE_CONTAINED_BY: &str = "RANGE_CONTAINED_BY";
    pub const RANGE_OVERLAP: &str = "RANGE_OVERLAP";
    pub const RANGE_LOWER: &str = "RANGE_LOWER";
    pub const RANGE_UPPER: &str = "RANGE_UPPER";
    pub const RANGE_IS_EMPTY: &str = "RANGE_IS_EMPTY";

    // Window / Ranking
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

    // UUID
    pub const GEN_RANDOM_UUID: &str = "GEN_RANDOM_UUID";

    // Geo / Spatial
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

    // Hashing / Encoding
    pub const HASH: &str = "HASH";
    pub const CRC32: &str = "CRC32";
    pub const HEX_ENCODE: &str = "HEX_ENCODE";
    pub const HEX_DECODE: &str = "HEX_DECODE";
}

impl fmt::Display for FuncDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl PartialEq<&str> for FuncDef {
    fn eq(&self, other: &&str) -> bool {
        self.name() == *other
    }
}

impl PartialEq<FuncDef> for &str {
    fn eq(&self, other: &FuncDef) -> bool {
        *self == other.name()
    }
}

impl From<&FuncDef> for Cow<'_, str> {
    fn from(def: &FuncDef) -> Self {
        Cow::Owned(def.name().to_string())
    }
}

/// Trait implemented by zero-sized structs representing well-known DOL functions.
pub trait DolFunc: Sized + 'static {
    /// The canonical DOL name for this function.
    const NAME: &'static str;
    /// The arity constraint.
    const ARITY: Arity;
    /// The function category.
    const KIND: FuncKind;

    /// Build a [`FuncDef`] from the trait constants.
    fn def() -> FuncDef {
        FuncDef::new_static(Self::NAME, Self::ARITY, Self::KIND)
    }
}

/// Declare a zero-sized struct implementing [`DolFunc`].
#[macro_export]
macro_rules! define_func {
    ($struct_name:ident, $name:expr, $arity:expr, $kind:expr) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $struct_name;

        impl $crate::tree::func::def::DolFunc for $struct_name {
            const NAME: &'static str = $name;
            const ARITY: $crate::tree::func::def::Arity = $arity;
            const KIND: $crate::tree::func::def::FuncKind = $kind;
        }
    };
}
