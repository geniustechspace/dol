//! Operators for DOL expressions.
//!
//! # Design
//!
//! **No `Not*` variants.** Negation is handled via `negated: bool` on
//! [`Expr::BinaryOp`](super::Expr::BinaryOp) — the same pattern already used
//! for `InList`, `InSubquery`, `Between`, `Exists`, and `IsNull`.
//! This keeps the variant count flat and the builder API chainable:
//! `field("name").like(lit("%foo%")).not()`.
//!
//! **`IsDistinctFrom` / `IsNotDistinctFrom`** stay as a pair because they
//! represent distinct SQL semantics (null-safe comparison), not simple boolean
//! negation of each other.

// ═══════════════════════════════════════════════════════════════════════════
// Binary operators
// ═══════════════════════════════════════════════════════════════════════════

/// Binary operators for expression composition.
///
/// Only universal, algebraic operators that apply across data systems.
/// Named operations (JSON navigation, range functions, collection mutation,
/// geo-distance) have been moved to [`FuncName`](super::func::FuncName).
///
/// Negatable operators (pattern, similarity) do **not** have `Not*`
/// counterparts — negation is expressed via `negated: bool` on the enclosing
/// `Expr::BinaryOp` node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BinOp {
    // ── Comparison ──────────────────────────────────────────────────────
    /// `=`
    Eq,
    /// `!=` / `<>`
    Ne,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Le,
    /// `>=`
    Ge,

    // ── Null-safe comparison ────────────────────────────────────────────
    /// `IS DISTINCT FROM` — null-safe inequality.
    IsDistinctFrom,
    /// `IS NOT DISTINCT FROM` — null-safe equality.
    IsNotDistinctFrom,

    // ── Arithmetic ──────────────────────────────────────────────────────
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%` / `MOD`
    Mod,

    // ── Logical ─────────────────────────────────────────────────────────
    /// `AND`
    And,
    /// `OR`
    Or,

    // ── Pattern matching (negation via `negated: bool`) ─────────────────
    /// `LIKE` / `NOT LIKE`
    Like,
    /// `ILIKE` / `NOT ILIKE` (case-insensitive LIKE)
    ILike,
    /// `SIMILAR TO` / `NOT SIMILAR TO` (SQL-standard regex)
    SimilarTo,
    /// POSIX regex match (`~` / `!~`)
    RegexMatch,
    /// POSIX regex match, case-insensitive (`~*` / `!~*`)
    RegexMatchInsensitive,
    /// Glob-style pattern match (SQLite `GLOB`)
    Glob,

    // ── String ──────────────────────────────────────────────────────────
    /// String concatenation (`||` / `CONCAT()` / `+` depending on dialect).
    Concat,

    // ── Bitwise ─────────────────────────────────────────────────────────
    /// `&` — bitwise AND.
    BitAnd,
    /// `|` — bitwise OR.
    BitOr,
    /// `^` / `#` — bitwise XOR.
    BitXor,
    /// `<<` — bit shift left.
    ShiftLeft,
    /// `>>` — bit shift right.
    ShiftRight,

    // ── Collection containment predicates ───────────────────────────────
    /// Array/collection contains element or sub-collection.
    ArrayContains,
    /// Array/collection is contained by another.
    ArrayContainedBy,
    /// Arrays/collections share at least one element.
    ArrayOverlap,
}

// ═══════════════════════════════════════════════════════════════════════════
// Unary operators
// ═══════════════════════════════════════════════════════════════════════════

/// Unary operators for expression composition.
///
/// Only three primitive operators remain here — everything else that used to
/// live here (IsNull, IsTrue, Abs, Sqrt, …) is either:
/// - A dedicated `Expr` variant (`Expr::IsNull`)  
/// - A named function (`FuncName::Abs`, `FuncName::Sqrt`, …)
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
// Ternary operator
// ═══════════════════════════════════════════════════════════════════════════

/// Ternary operators — operations that are naturally three-operand.
///
/// `Between` is the canonical example: `expr BETWEEN low AND high`.
/// Keeping it here (alongside the existing `Expr::Between` variant) allows
/// generic lowering paths while the dedicated variant remains for ergonomic
/// constructor use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TernaryOp {
    /// `expr [NOT] BETWEEN low AND high`
    Between,
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
