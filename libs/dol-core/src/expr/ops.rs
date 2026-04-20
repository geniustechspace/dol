//! Operators for DOL expressions.
//!
//! Unary operators and quantifiers for DOL expression composition.

// ═══════════════════════════════════════════════════════════════════════════
// Unary operators
// ═══════════════════════════════════════════════════════════════════════════

/// Unary operators for expression composition.
///
/// Only three primitive operators remain here — everything else that used to
/// live here (IsNull, IsTrue, Abs, Sqrt, …) is either:
/// - A dedicated `Expr` variant (`Expr::IsNull`)
/// - A named function (`FuncDef::ABS`, `FuncDef::SQRT`, …)
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
