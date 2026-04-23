//! Operators for DOL expressions.

// ═══════════════════════════════════════════════════════════════════════════
// Unary operators
// ═══════════════════════════════════════════════════════════════════════════

/// Unary operators for expression composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UnaryOp {
    /// `NOT expr` — boolean negation.
    Not,
    /// `-expr` — arithmetic negation.
    Neg,
    /// `~expr` — bitwise complement.
    BitNot,
    /// `expr IS NULL`
    IsNull,
    /// `expr IS NOT NULL`
    IsNotNull,
}
