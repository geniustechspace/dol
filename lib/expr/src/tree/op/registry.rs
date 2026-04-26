//! Registry of all well-known DOL operators as zero-sized structs.
//!
//! Each struct implements [`DolOp`](super::meta::DolOp) via the
//! [`define_op!`](crate::define_op) macro, providing a canonical name and
//! operator kind.

use super::meta::OpKind;
use crate::define_op;

// ═══════════════════════════════════════════════════════════════════════════
// Comparison operators
// ═══════════════════════════════════════════════════════════════════════════

define_op!(OpEq, "EQ", OpKind::Comparison);
define_op!(OpNe, "NE", OpKind::Comparison);
define_op!(OpLt, "LT", OpKind::Comparison);
define_op!(OpGt, "GT", OpKind::Comparison);
define_op!(OpLe, "LE", OpKind::Comparison);
define_op!(OpGe, "GE", OpKind::Comparison);

// ═══════════════════════════════════════════════════════════════════════════
// Null-safe comparison operators
// ═══════════════════════════════════════════════════════════════════════════

define_op!(OpIsDistinctFrom, "IS_DISTINCT_FROM", OpKind::NullSafe);
define_op!(
    OpIsNotDistinctFrom,
    "IS_NOT_DISTINCT_FROM",
    OpKind::NullSafe
);

// ═══════════════════════════════════════════════════════════════════════════
// Arithmetic operators
// ═══════════════════════════════════════════════════════════════════════════

define_op!(OpAdd, "ADD", OpKind::Arithmetic);
define_op!(OpSub, "SUB", OpKind::Arithmetic);
define_op!(OpMul, "MUL", OpKind::Arithmetic);
define_op!(OpDiv, "DIV", OpKind::Arithmetic);
define_op!(OpMod, "MOD", OpKind::Arithmetic);

// ═══════════════════════════════════════════════════════════════════════════
// Logical operators
// ═══════════════════════════════════════════════════════════════════════════

define_op!(OpAnd, "AND", OpKind::Logical);
define_op!(OpOr, "OR", OpKind::Logical);

// ═══════════════════════════════════════════════════════════════════════════
// Pattern matching operators
// ═══════════════════════════════════════════════════════════════════════════

define_op!(OpLike, "LIKE", OpKind::Pattern);
define_op!(OpIlike, "ILIKE", OpKind::Pattern);
define_op!(OpSimilarTo, "SIMILAR_TO", OpKind::Pattern);
define_op!(OpRegexMatch, "REGEX_MATCH", OpKind::Pattern);
define_op!(
    OpRegexMatchInsensitive,
    "REGEX_MATCH_INSENSITIVE",
    OpKind::Pattern
);
define_op!(OpGlob, "GLOB", OpKind::Pattern);

// ═══════════════════════════════════════════════════════════════════════════
// String operators
// ═══════════════════════════════════════════════════════════════════════════

define_op!(OpConcat, "CONCAT", OpKind::StringOp);

// ═══════════════════════════════════════════════════════════════════════════
// Bitwise operators
// ═══════════════════════════════════════════════════════════════════════════

define_op!(OpBitAnd, "BIT_AND", OpKind::Bitwise);
define_op!(OpBitOr, "BIT_OR", OpKind::Bitwise);
define_op!(OpBitXor, "BIT_XOR", OpKind::Bitwise);
define_op!(OpShiftLeft, "SHIFT_LEFT", OpKind::Bitwise);
define_op!(OpShiftRight, "SHIFT_RIGHT", OpKind::Bitwise);

// ═══════════════════════════════════════════════════════════════════════════
// Collection containment operators
// ═══════════════════════════════════════════════════════════════════════════

define_op!(OpContains, "CONTAINS", OpKind::Collection);
define_op!(OpContainedBy, "CONTAINED_BY", OpKind::Collection);
define_op!(OpOverlap, "OVERLAP", OpKind::Collection);
