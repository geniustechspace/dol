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
/// Negatable operators (pattern, similarity, range) do **not** have `Not*`
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
    /// `ILIKE` / `NOT ILIKE` (case-insensitive)
    ILike,
    /// `SIMILAR TO` / `NOT SIMILAR TO` (SQL-standard regex)
    SimilarTo,
    /// `~` / `!~` (POSIX regex match)
    RegexMatch,
    /// `~*` / `!~*` (POSIX regex match, case-insensitive)
    RegexMatchInsensitive,
    /// Glob-style pattern match (SQLite `GLOB`, or lowered to `LIKE`)
    Glob,

    // ── String ──────────────────────────────────────────────────────────
    /// `||` / `CONCAT()` / `+` depending on dialect.
    Concat,
    /// `STARTS WITH` / `^@` — prefix match.
    StartsWith,
    /// `CONTAINS` — substring containment.
    Contains,

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

    // ── Array / Collection ──────────────────────────────────────────────
    /// `@>` — array/jsonb contains.
    ArrayContains,
    /// `<@` — array/jsonb contained-by.
    ArrayContainedBy,
    /// `&&` — array overlap (share elements).
    ArrayOverlap,

    // ── JSON / Document ─────────────────────────────────────────────────
    /// `->` — JSON field access (returns JSON).
    JsonGet,
    /// `->>` — JSON field access (returns text).
    JsonGetText,
    /// `#>` — JSON path access (returns JSON).
    JsonPath,
    /// `#>>` — JSON path access (returns text).
    JsonPathText,
    /// `?` — JSON key exists (negation via `negated`).
    JsonHasKey,
    /// `?|` — JSON has any of the keys.
    JsonHasAnyKey,
    /// `?&` — JSON has all of the keys.
    JsonHasAllKeys,

    // ── Range ───────────────────────────────────────────────────────────
    /// `@>` on range types — range contains element/range.
    RangeContains,
    /// `<@` on range types — range is contained by.
    RangeContainedBy,
    /// `&&` on range types — ranges overlap.
    RangeOverlap,

    // ── Set / Collection ops (value-level, not query-level) ─────────────
    /// `MERGE` — deep-merge two structured values.
    Merge,
    /// `APPEND` — append to array/list.
    Append,
    /// `PREPEND` — prepend to array/list.
    Prepend,
    /// `REMOVE` — remove key/element from collection.
    RemoveKey,

    // ── Geo / Spatial ───────────────────────────────────────────────────
    /// Distance operator (`<->` in PostGIS).
    Distance,
}

// ═══════════════════════════════════════════════════════════════════════════
// Unary operators
// ═══════════════════════════════════════════════════════════════════════════

/// Unary operators for expression composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UnaryOp {
    // ── Logical ─────────────────────────────────────────────────────────
    /// `NOT expr`
    Not,
    /// `-expr` — arithmetic negation.
    Neg,

    // ── Boolean tests (IS …) ────────────────────────────────────────────
    /// `IS NULL`
    IsNull,
    /// `IS NOT NULL`
    IsNotNull,
    /// `IS TRUE`
    IsTrue,
    /// `IS NOT TRUE`
    IsNotTrue,
    /// `IS FALSE`
    IsFalse,
    /// `IS NOT FALSE`
    IsNotFalse,
    /// `IS UNKNOWN`
    IsUnknown,
    /// `IS NOT UNKNOWN`
    IsNotUnknown,

    // ── Bitwise ─────────────────────────────────────────────────────────
    /// `~` — bitwise NOT.
    BitNot,

    // ── Math ────────────────────────────────────────────────────────────
    /// `|/` — square root (Postgres).
    Sqrt,
    /// `||/` — cube root (Postgres).
    CubeRoot,
    /// `@` — absolute value (Postgres).
    Abs,
    /// `!` — factorial (Postgres).
    Factorial,
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
