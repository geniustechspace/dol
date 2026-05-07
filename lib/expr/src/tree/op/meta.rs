//! Operator metadata and traits for well-known DOL operators.

use alloc::boxed::Box;

use core::fmt;

use super::super::compact_name::CompactName;

/// Classification of a DOL binary operator.
///
/// This enum is intentionally a 1:1 mirror of the
/// [`BinOp`](crate::expr::BinOp) tier (Tier A in the four-tier rule
/// — see `lib/expr/src/expr.rs` head doc). Categories that suggested
/// tier confusion (`Collection`, regex/glob in `Pattern`) were removed
/// when those constructs moved to function calls in
/// `tree::func::registry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum OpCategory {
    /// Comparison operators: `=`, `!=`, `<`, `>`, `<=`, `>=`.
    Comparison,
    /// Null-safe comparison: `IS DISTINCT FROM`, `IS NOT DISTINCT FROM`.
    NullSafe,
    /// Arithmetic operators: `+`, `-`, `*`, `/`, `%`.
    Arithmetic,
    /// Logical operators: `AND`, `OR`.
    Logical,
    /// Pattern matching operators: `LIKE`, `ILIKE`, `SIMILAR TO`.
    /// Regex and glob variants live in `tree::func::registry` as
    /// `REGEX_MATCH` / `REGEX_IMATCH` / `GLOB_MATCH` — they are not
    /// universally infix and so do not qualify as `BinOp`s.
    Pattern,
    /// String operators: `||` (concatenation).
    StringOp,
    /// Bitwise operators: `&`, `|`, `^`, `<<`, `>>`.
    Bitwise,
}

/// A rich operator definition: name + kind.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct OpDef {
    name: CompactName,
    kind: OpCategory,
}

impl OpDef {
    /// Create a new operator definition (used by the `DolOp` trait).
    pub const fn new_static(name: &'static str, kind: OpCategory) -> Self {
        Self {
            name: CompactName::Static(name),
            kind,
        }
    }

    /// Create a custom operator definition.
    pub fn custom(name: impl Into<Box<str>>, kind: OpCategory) -> Self {
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
    pub fn kind(&self) -> OpCategory {
        self.kind
    }

    // Comparison
    pub const EQ: &str = "EQ";
    pub const NE: &str = "NE";
    pub const LT: &str = "LT";
    pub const GT: &str = "GT";
    pub const LE: &str = "LE";
    pub const GE: &str = "GE";

    // Null-safe comparison
    pub const IS_DISTINCT_FROM: &str = "IS_DISTINCT_FROM";
    pub const IS_NOT_DISTINCT_FROM: &str = "IS_NOT_DISTINCT_FROM";

    // Arithmetic
    pub const ADD: &str = "ADD";
    pub const SUB: &str = "SUB";
    pub const MUL: &str = "MUL";
    pub const DIV: &str = "DIV";
    pub const MOD: &str = "MOD";

    // Logical
    pub const AND: &str = "AND";
    pub const OR: &str = "OR";

    // Pattern
    pub const LIKE: &str = "LIKE";
    pub const ILIKE: &str = "ILIKE";
    pub const SIMILAR_TO: &str = "SIMILAR_TO";

    // String
    pub const CONCAT: &str = "CONCAT";

    // Bitwise
    pub const BIT_AND: &str = "BIT_AND";
    pub const BIT_OR: &str = "BIT_OR";
    pub const BIT_XOR: &str = "BIT_XOR";
    pub const SHIFT_LEFT: &str = "SHIFT_LEFT";
    pub const SHIFT_RIGHT: &str = "SHIFT_RIGHT";
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

/// Trait implemented by zero-sized structs representing well-known DOL operators.
pub trait DolOp: Sized + 'static {
    /// The canonical DOL name for this operator.
    const NAME: &'static str;
    /// The operator category.
    const KIND: OpCategory;

    /// Build an [`OpDef`] from the trait constants.
    fn def() -> OpDef {
        OpDef::new_static(Self::NAME, Self::KIND)
    }
}

/// Declare a zero-sized struct implementing [`DolOp`].
#[macro_export]
macro_rules! define_op {
    ($struct_name:ident, $name:expr, $kind:expr) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $struct_name;

        impl $crate::tree::op::meta::DolOp for $struct_name {
            const NAME: &'static str = $name;
            const KIND: $crate::tree::op::meta::OpCategory = $kind;
        }
    };
}
