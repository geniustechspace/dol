//! Registry of all well-known DOL operators as zero-sized structs.
//!
//! Each struct implements [`DolOp`](super::meta::DolOp) via the
//! [`define_op!`](crate::define_op) macro, providing a canonical name and
//! operator kind.

use super::meta::OpCategory;
use crate::define_op;

// ═══════════════════════════════════════════════════════════════════════════
// Comparison operators
// ═══════════════════════════════════════════════════════════════════════════

define_op!(OpEq, "EQ", OpCategory::Comparison);
define_op!(OpNe, "NE", OpCategory::Comparison);
define_op!(OpLt, "LT", OpCategory::Comparison);
define_op!(OpGt, "GT", OpCategory::Comparison);
define_op!(OpLe, "LE", OpCategory::Comparison);
define_op!(OpGe, "GE", OpCategory::Comparison);

// ═══════════════════════════════════════════════════════════════════════════
// Null-safe comparison operators
// ═══════════════════════════════════════════════════════════════════════════

define_op!(OpIsDistinctFrom, "IS_DISTINCT_FROM", OpCategory::NullSafe);
define_op!(
    OpIsNotDistinctFrom,
    "IS_NOT_DISTINCT_FROM",
    OpCategory::NullSafe
);

// ═══════════════════════════════════════════════════════════════════════════
// Arithmetic operators
// ═══════════════════════════════════════════════════════════════════════════

define_op!(OpAdd, "ADD", OpCategory::Arithmetic);
define_op!(OpSub, "SUB", OpCategory::Arithmetic);
define_op!(OpMul, "MUL", OpCategory::Arithmetic);
define_op!(OpDiv, "DIV", OpCategory::Arithmetic);
define_op!(OpMod, "MOD", OpCategory::Arithmetic);

// ═══════════════════════════════════════════════════════════════════════════
// Logical operators
// ═══════════════════════════════════════════════════════════════════════════

define_op!(OpAnd, "AND", OpCategory::Logical);
define_op!(OpOr, "OR", OpCategory::Logical);

// ═══════════════════════════════════════════════════════════════════════════
// Pattern matching operators
// ═══════════════════════════════════════════════════════════════════════════
//
// `LIKE`, `ILIKE`, and `SIMILAR TO` earn `BinOp` slots — they are universally
// rendered infix on every relational backend with a stable keyword spelling.
// `REGEX_MATCH`, `REGEX_MATCH_INSENSITIVE`, and `GLOB` do **not** qualify
// (Postgres `~`/`~*`, MySQL `REGEXP`, SQLite `GLOB` — three different infix
// spellings); they live in `tree::func::registry` as named functions.

define_op!(OpLike, "LIKE", OpCategory::Pattern);
define_op!(OpIlike, "ILIKE", OpCategory::Pattern);
define_op!(OpSimilarTo, "SIMILAR_TO", OpCategory::Pattern);

// ═══════════════════════════════════════════════════════════════════════════
// String operators
// ═══════════════════════════════════════════════════════════════════════════

define_op!(OpConcat, "CONCAT", OpCategory::StringOp);

// ═══════════════════════════════════════════════════════════════════════════
// Bitwise operators
// ═══════════════════════════════════════════════════════════════════════════

define_op!(OpBitAnd, "BIT_AND", OpCategory::Bitwise);
define_op!(OpBitOr, "BIT_OR", OpCategory::Bitwise);
define_op!(OpBitXor, "BIT_XOR", OpCategory::Bitwise);
define_op!(OpShiftLeft, "SHIFT_LEFT", OpCategory::Bitwise);
define_op!(OpShiftRight, "SHIFT_RIGHT", OpCategory::Bitwise);

// ═══════════════════════════════════════════════════════════════════════════
// Collection containment operators — REMOVED
// ═══════════════════════════════════════════════════════════════════════════
//
// `CONTAINS` (`@>`), `CONTAINED_BY` (`<@`), and `OVERLAP` (`&&`) are no
// longer first-class operators. `CONTAINED_BY` semantically conflicted with
// `IN` (`x <@ array_of(a,b,c)` and `x IN (a,b,c)` spell the same membership
// test), and none of these are universally infix across backends. Use the
// named functions `contains(haystack, needle)`, `overlaps(a, b)`, etc., in
// `tree::func::registry` instead.
