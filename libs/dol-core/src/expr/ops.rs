//! Operators for DOL expressions.
//!
//! # Design
//!
//! [`OpId`] is a string-keyed newtype for binary operators, with well-known
//! constants for every standard operator. Custom operators use `OpId::new()`.
//!
//! **No `Not*` variants.** Negation is handled via `negated: bool` on
//! [`Expr::BinaryOp`](super::Expr::BinaryOp) — the same pattern already used
//! for `InList`, `InSubquery`, `Between`, `Exists`, and `IsNull`.
//!
//! **`IS_DISTINCT_FROM` / `IS_NOT_DISTINCT_FROM`** stay as a pair because they
//! represent distinct semantics (null-safe comparison), not simple boolean
//! negation of each other.

use core::fmt;

// ═══════════════════════════════════════════════════════════════════════════
// OpId — extensible binary operator identifier
// ═══════════════════════════════════════════════════════════════════════════

/// Binary operator identifier for expression composition.
///
/// All universal, algebraic operators have `&str` constants.
/// Custom operators use [`OpId::new()`].
///
/// Negatable operators (pattern, similarity) do **not** have `NOT_*`
/// counterparts — negation is expressed via `negated: bool` on the enclosing
/// `Expr::BinaryOp` node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpId(Box<str>);

impl OpId {
    /// Create an operator identifier from any string.
    pub fn new(name: impl Into<Box<str>>) -> Self {
        Self(name.into())
    }

    /// Return the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    // ── Comparison ──────────────────────────────────────────────────────
    /// `=`
    pub const EQ: &str = "EQ";
    /// `!=` / `<>`
    pub const NE: &str = "NE";
    /// `<`
    pub const LT: &str = "LT";
    /// `>`
    pub const GT: &str = "GT";
    /// `<=`
    pub const LE: &str = "LE";
    /// `>=`
    pub const GE: &str = "GE";

    // ── Null-safe comparison ────────────────────────────────────────────
    /// `IS DISTINCT FROM` — null-safe inequality.
    pub const IS_DISTINCT_FROM: &str = "IS_DISTINCT_FROM";
    /// `IS NOT DISTINCT FROM` — null-safe equality.
    pub const IS_NOT_DISTINCT_FROM: &str = "IS_NOT_DISTINCT_FROM";

    // ── Arithmetic ──────────────────────────────────────────────────────
    /// `+`
    pub const ADD: &str = "ADD";
    /// `-`
    pub const SUB: &str = "SUB";
    /// `*`
    pub const MUL: &str = "MUL";
    /// `/`
    pub const DIV: &str = "DIV";
    /// `%` / `MOD`
    pub const MOD: &str = "MOD";

    // ── Logical ─────────────────────────────────────────────────────────
    /// `AND`
    pub const AND: &str = "AND";
    /// `OR`
    pub const OR: &str = "OR";

    // ── Pattern matching (negation via `negated: bool`) ─────────────────
    /// `LIKE` / `NOT LIKE`
    pub const LIKE: &str = "LIKE";
    /// `ILIKE` / `NOT ILIKE` (case-insensitive LIKE)
    pub const ILIKE: &str = "ILIKE";
    /// `SIMILAR TO` / `NOT SIMILAR TO` (SQL-standard regex)
    pub const SIMILAR_TO: &str = "SIMILAR_TO";
    /// POSIX regex match (`~` / `!~`)
    pub const REGEX_MATCH: &str = "REGEX_MATCH";
    /// POSIX regex match, case-insensitive (`~*` / `!~*`)
    pub const REGEX_MATCH_INSENSITIVE: &str = "REGEX_MATCH_INSENSITIVE";
    /// Glob-style pattern match (SQLite `GLOB`)
    pub const GLOB: &str = "GLOB";

    // ── String ──────────────────────────────────────────────────────────
    /// String concatenation (`||` / `CONCAT()` / `+` depending on dialect).
    pub const CONCAT: &str = "CONCAT";

    // ── Bitwise ─────────────────────────────────────────────────────────
    /// `&` — bitwise AND.
    pub const BIT_AND: &str = "BIT_AND";
    /// `|` — bitwise OR.
    pub const BIT_OR: &str = "BIT_OR";
    /// `^` / `#` — bitwise XOR.
    pub const BIT_XOR: &str = "BIT_XOR";
    /// `<<` — bit shift left.
    pub const SHIFT_LEFT: &str = "SHIFT_LEFT";
    /// `>>` — bit shift right.
    pub const SHIFT_RIGHT: &str = "SHIFT_RIGHT";

    // ── Collection containment predicates ───────────────────────────────
    /// Array/collection contains element or sub-collection.
    pub const CONTAINS: &str = "CONTAINS";
    /// Array/collection is contained by another.
    pub const CONTAINED_BY: &str = "CONTAINED_BY";
    /// Arrays/collections share at least one element.
    pub const OVERLAP: &str = "OVERLAP";
}

impl fmt::Display for OpId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<&str> for OpId {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_ref() == *other
    }
}

impl PartialEq<OpId> for &str {
    fn eq(&self, other: &OpId) -> bool {
        *self == other.0.as_ref()
    }
}

impl From<&str> for OpId {
    fn from(s: &str) -> Self {
        Self(s.into())
    }
}

/// Backward-compatibility alias.
#[deprecated(note = "renamed to OpId — use OpId instead")]
pub type BinOp = OpId;

// ═══════════════════════════════════════════════════════════════════════════
// Unary operators
// ═══════════════════════════════════════════════════════════════════════════

/// Unary operators for expression composition.
///
/// Only three primitive operators remain here — everything else that used to
/// live here (IsNull, IsTrue, Abs, Sqrt, …) is either:
/// - A dedicated `Expr` variant (`Expr::IsNull`)
/// - A named function (`FuncId::ABS`, `FuncId::SQRT`, …)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UnaryOp {
    /// `NOT expr` — boolean negation.
    Not,
    /// `-expr` — arithmetic negation.
    Neg,
    /// `~expr` — bitwise complement.
    BitNot,
}

// ═══════════════════════════════════════════════════════════════════════════
// Quantifier (for ANY / ALL style comparisons)
// ═══════════════════════════════════════════════════════════════════════════

/// Quantifier for quantified comparisons: `expr op ANY(...)` / `expr op ALL(...)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Quantifier {
    /// `ANY` / `SOME`
    Any,
    /// `ALL`
    All,
}
