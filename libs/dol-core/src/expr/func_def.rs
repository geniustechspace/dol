//! Rich function definitions — the extensible, metadata-carrying function catalogue.
//!
//! [`FuncDef`] replaces raw string identifiers with structured definitions that
//! carry arity constraints and a function kind, enabling validation, category-aware
//! DSL methods, and zero-allocation construction for well-known functions.
//!
//! # Design
//!
//! - **[`CompactName`]** stores either a `&'static str` (zero-alloc for well-known
//!   functions) or an owned `Box<str>` (for custom/runtime functions).
//! - **[`Arity`]** describes the expected argument count.
//! - **[`FuncKind`]** classifies the function (scalar, aggregate, window, etc.).
//! - **[`FuncDef`]** combines all three.
//!
//! # Usage
//!
//! Well-known functions are constructed via the [`DolFunc`] trait on zero-sized
//! structs. Custom functions use [`FuncDef::custom()`].

use core::fmt;
use std::borrow::Cow;

// ═══════════════════════════════════════════════════════════════════════════
// CompactName
// ═══════════════════════════════════════════════════════════════════════════

/// A name that's either a static string (well-known, zero-alloc) or an owned
/// heap string (custom/runtime).
///
/// Comparison and hashing operate on the string content regardless of variant.
#[derive(Debug, Clone)]
pub enum CompactName {
    /// A well-known name — `&'static str`, zero heap allocation.
    Static(&'static str),
    /// A custom/runtime name — owned heap string.
    Owned(Box<str>),
}

impl CompactName {
    /// Return the string content.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Static(s) => s,
            Self::Owned(s) => s,
        }
    }
}

impl PartialEq for CompactName {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for CompactName {}

impl std::hash::Hash for CompactName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl fmt::Display for CompactName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&'static str> for CompactName {
    fn from(s: &'static str) -> Self {
        Self::Static(s)
    }
}

impl From<Box<str>> for CompactName {
    fn from(s: Box<str>) -> Self {
        Self::Owned(s)
    }
}

impl From<String> for CompactName {
    fn from(s: String) -> Self {
        Self::Owned(s.into_boxed_str())
    }
}

// ── Serde ──────────────────────────────────────────────────────────────────

#[cfg(feature = "serde")]
impl serde::Serialize for CompactName {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for CompactName {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::Owned(s.into_boxed_str()))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Arity
// ═══════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════
// FuncKind
// ═══════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════
// FuncDef
// ═══════════════════════════════════════════════════════════════════════════

/// A rich function definition: name + arity + kind.
///
/// Stored in `Expr::Func` instead of the old `FuncId`. Carries metadata
/// that enables arity validation and category-aware rendering.
///
/// # Construction
///
/// Well-known functions use `DolFunc::def()` (zero heap allocation):
///
/// ```ignore
/// use dol_core::expr::func_def::{DolFunc, FuncDef};
/// use dol_core::expr::func_registry::Count;
///
/// let def: FuncDef = Count::def();
/// assert_eq!(def.name(), "COUNT");
/// ```
///
/// Custom functions use `FuncDef::custom()`:
///
/// ```ignore
/// let def = FuncDef::custom("MY_FUNC");
/// ```
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

    /// Create a custom function definition (for user-defined / backend-specific functions).
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

    /// Returns `true` if this is a well-known "no-parens" SQL keyword
    /// (e.g. `CURRENT_DATE`).
    pub fn is_no_parens_keyword(&self) -> bool {
        matches!(
            self.name(),
            "CURRENT_DATE" | "CURRENT_TIME" | "CURRENT_TIMESTAMP"
        )
    }
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

// ── Conversion from legacy FuncId ──────────────────────────────────────────

impl From<super::FuncId> for FuncDef {
    fn from(id: super::FuncId) -> Self {
        Self {
            name: CompactName::Owned(id.as_str().into()),
            arity: Arity::Any,
            kind: FuncKind::Scalar,
        }
    }
}

impl From<&FuncDef> for Cow<'_, str> {
    fn from(def: &FuncDef) -> Self {
        Cow::Owned(def.name().to_string())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DolFunc trait
// ═══════════════════════════════════════════════════════════════════════════

/// Trait implemented by zero-sized structs representing well-known DOL functions.
///
/// Each implementor provides a constant `NAME`, `ARITY`, and `KIND` that are
/// combined into a [`FuncDef`] via [`DolFunc::def()`].
///
/// # Extension pattern
///
/// Backends add behavior per-function without touching this trait:
///
/// ```ignore
/// pub trait PostgresFunc: DolFunc {
///     fn pg_name() -> &'static str { Self::NAME }
/// }
/// impl PostgresFunc for PadLeft {
///     fn pg_name() -> &'static str { "LPAD" }
/// }
/// ```
pub trait DolFunc: Sized + 'static {
    /// The canonical DOL name for this function (e.g. `"COUNT"`, `"LOWER"`).
    const NAME: &'static str;
    /// The arity constraint.
    const ARITY: Arity;
    /// The function category.
    const KIND: FuncKind;

    /// Build a [`FuncDef`] from the trait constants (zero heap allocation).
    fn def() -> FuncDef {
        FuncDef::new_static(Self::NAME, Self::ARITY, Self::KIND)
    }
}

/// Declare a zero-sized struct implementing [`DolFunc`].
///
/// # Example
///
/// ```ignore
/// define_func!(Count, "COUNT", Arity::Exact(1), FuncKind::Aggregate);
/// ```
#[macro_export]
macro_rules! define_func {
    ($struct_name:ident, $name:expr, $arity:expr, $kind:expr) => {
        /// Zero-sized marker struct for the well-known DOL function.
        #[derive(Debug, Clone, Copy)]
        pub struct $struct_name;

        impl $crate::expr::func_def::DolFunc for $struct_name {
            const NAME: &'static str = $name;
            const ARITY: $crate::expr::func_def::Arity = $arity;
            const KIND: $crate::expr::func_def::FuncKind = $kind;
        }
    };
}

// ═══════════════════════════════════════════════════════════════════════════
// OpKind
// ═══════════════════════════════════════════════════════════════════════════

/// Classification of a DOL binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OpKind {
    /// Comparison operators: `=`, `!=`, `<`, `>`, `<=`, `>=`.
    Comparison,
    /// Null-safe comparison: `IS DISTINCT FROM`, `IS NOT DISTINCT FROM`.
    NullSafe,
    /// Arithmetic operators: `+`, `-`, `*`, `/`, `%`.
    Arithmetic,
    /// Logical operators: `AND`, `OR`.
    Logical,
    /// Pattern matching operators: `LIKE`, `ILIKE`, `SIMILAR TO`, `~`, `~*`, `GLOB`.
    Pattern,
    /// String operators: `||` (concatenation).
    StringOp,
    /// Bitwise operators: `&`, `|`, `^`, `<<`, `>>`.
    Bitwise,
    /// Collection containment operators: `@>`, `<@`, `&&`.
    Collection,
}

// ═══════════════════════════════════════════════════════════════════════════
// OpDef
// ═══════════════════════════════════════════════════════════════════════════

/// A rich operator definition: name + kind.
///
/// Stored in `Expr::BinaryOp` instead of the old `OpId`. Carries metadata
/// that enables category-aware rendering.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpDef {
    name: CompactName,
    kind: OpKind,
}

impl OpDef {
    /// Create a new operator definition (used by the `DolOp` trait).
    pub const fn new_static(name: &'static str, kind: OpKind) -> Self {
        Self {
            name: CompactName::Static(name),
            kind,
        }
    }

    /// Create a custom operator definition.
    pub fn custom(name: impl Into<Box<str>>, kind: OpKind) -> Self {
        Self {
            name: CompactName::Owned(name.into()),
            kind,
        }
    }

    /// Return the operator name.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Return the operator kind.
    pub fn kind(&self) -> OpKind {
        self.kind
    }
}

impl fmt::Display for OpDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl PartialEq<&str> for OpDef {
    fn eq(&self, other: &&str) -> bool {
        self.name() == *other
    }
}

impl PartialEq<OpDef> for &str {
    fn eq(&self, other: &OpDef) -> bool {
        *self == other.name()
    }
}

// ── Conversion from legacy OpId ────────────────────────────────────────────

impl From<super::OpId> for OpDef {
    fn from(id: super::OpId) -> Self {
        // Attempt to recognize well-known ops for proper OpKind assignment.
        let kind = match id.as_str() {
            "EQ" | "NE" | "LT" | "GT" | "LE" | "GE" => OpKind::Comparison,
            "IS_DISTINCT_FROM" | "IS_NOT_DISTINCT_FROM" => OpKind::NullSafe,
            "ADD" | "SUB" | "MUL" | "DIV" | "MOD" => OpKind::Arithmetic,
            "AND" | "OR" => OpKind::Logical,
            "LIKE" | "ILIKE" | "SIMILAR_TO" | "REGEX_MATCH" | "REGEX_MATCH_INSENSITIVE"
            | "GLOB" => OpKind::Pattern,
            "CONCAT" => OpKind::StringOp,
            "BIT_AND" | "BIT_OR" | "BIT_XOR" | "SHIFT_LEFT" | "SHIFT_RIGHT" => OpKind::Bitwise,
            "CONTAINS" | "CONTAINED_BY" | "OVERLAP" => OpKind::Collection,
            _ => OpKind::Comparison, // fallback
        };
        Self {
            name: CompactName::Owned(id.as_str().into()),
            kind,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DolOp trait
// ═══════════════════════════════════════════════════════════════════════════

/// Trait implemented by zero-sized structs representing well-known DOL operators.
pub trait DolOp: Sized + 'static {
    /// The canonical DOL name for this operator (e.g. `"EQ"`, `"ADD"`).
    const NAME: &'static str;
    /// The operator category.
    const KIND: OpKind;

    /// Build an [`OpDef`] from the trait constants (zero heap allocation).
    fn def() -> OpDef {
        OpDef::new_static(Self::NAME, Self::KIND)
    }
}

/// Declare a zero-sized struct implementing [`DolOp`].
#[macro_export]
macro_rules! define_op {
    ($struct_name:ident, $name:expr, $kind:expr) => {
        /// Zero-sized marker struct for the well-known DOL operator.
        #[derive(Debug, Clone, Copy)]
        pub struct $struct_name;

        impl $crate::expr::func_def::DolOp for $struct_name {
            const NAME: &'static str = $name;
            const KIND: $crate::expr::func_def::OpKind = $kind;
        }
    };
}
