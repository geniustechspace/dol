//! Operators for DOL expressions.
//!
//! Submodules:
//! - [`meta`]     — operator metadata: kind, arity, symbol
//! - [`registry`] — built-in operator registry

pub mod meta;
pub mod registry;

// ═══════════════════════════════════════════════════════════════════════════
// Unary operators
// ═══════════════════════════════════════════════════════════════════════════

/// Unary operators for expression composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
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
