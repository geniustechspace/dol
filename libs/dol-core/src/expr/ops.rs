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
    pub const EQ: &str = "eq";
    /// `!=` / `<>`
    pub const NE: &str = "ne";
    /// `<`
    pub const LT: &str = "lt";
    /// `>`
    pub const GT: &str = "gt";
    /// `<=`
    pub const LE: &str = "le";
    /// `>=`
    pub const GE: &str = "ge";

    // ── Null-safe comparison ────────────────────────────────────────────
    /// `IS DISTINCT FROM` — null-safe inequality.
    pub const IS_DISTINCT_FROM: &str = "is_distinct_from";
    /// `IS NOT DISTINCT FROM` — null-safe equality.
    pub const IS_NOT_DISTINCT_FROM: &str = "is_not_distinct_from";

    // ── Arithmetic ──────────────────────────────────────────────────────
    /// `+`
    pub const ADD: &str = "add";
    /// `-`
    pub const SUB: &str = "sub";
    /// `*`
    pub const MUL: &str = "mul";
    /// `/`
    pub const DIV: &str = "div";
    /// `%` / `MOD`
    pub const MOD: &str = "mod";

    // ── Logical ─────────────────────────────────────────────────────────
    /// `AND`
    pub const AND: &str = "and";
    /// `OR`
    pub const OR: &str = "or";

    // ── Pattern matching (negation via `negated: bool`) ─────────────────
    /// `LIKE` / `NOT LIKE`
    pub const LIKE: &str = "like";
    /// `ILIKE` / `NOT ILIKE` (case-insensitive LIKE)
    pub const ILIKE: &str = "ilike";
    /// `SIMILAR TO` / `NOT SIMILAR TO` (SQL-standard regex)
    pub const SIMILAR_TO: &str = "similar_to";
    /// POSIX regex match (`~` / `!~`)
    pub const REGEX_MATCH: &str = "regex_match";
    /// POSIX regex match, case-insensitive (`~*` / `!~*`)
    pub const REGEX_MATCH_INSENSITIVE: &str = "regex_match_insensitive";
    /// Glob-style pattern match (SQLite `GLOB`)
    pub const GLOB: &str = "glob";

    // ── String ──────────────────────────────────────────────────────────
    /// String concatenation (`||` / `CONCAT()` / `+` depending on dialect).
    pub const CONCAT: &str = "concat";

    // ── Bitwise ─────────────────────────────────────────────────────────
    /// `&` — bitwise AND.
    pub const BIT_AND: &str = "bit_and";
    /// `|` — bitwise OR.
    pub const BIT_OR: &str = "bit_or";
    /// `^` / `#` — bitwise XOR.
    pub const BIT_XOR: &str = "bit_xor";
    /// `<<` — bit shift left.
    pub const SHIFT_LEFT: &str = "shift_left";
    /// `>>` — bit shift right.
    pub const SHIFT_RIGHT: &str = "shift_right";

    // ── Collection containment predicates ───────────────────────────────
    /// Array/collection contains element or sub-collection.
    pub const CONTAINS: &str = "contains";
    /// Array/collection is contained by another.
    pub const CONTAINED_BY: &str = "contained_by";
    /// Arrays/collections share at least one element.
    pub const OVERLAP: &str = "overlap";
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
