//! Tree-DSL leaf metadata: [`OpDef`] / [`FuncDef`] and supporting
//! enums.
//!
//! Per `dol-rewrite-plan-v2.md` §8.2 — first slice. The `Expr<'a>`
//! enum (and its `Binary { op: OpDef, … }` / `Call { func: FuncDef, …
//! }` variants) lands in M3c-β; this slice ships only the leaf
//! metadata so it can be reviewed in isolation.
//!
//! ## Why these are *tree-side* metadata
//!
//! [`crate::expr::ops::BinOp`] (in `expr::ops`) is the **wire-side**
//! u16 opcode that lives inside the 16-byte [`crate::expr::node::ExprNode`].
//! [`OpDef`] is the **tree-side** counterpart — it carries an
//! identifying [`Name`] plus an [`OpCategory`] for downstream
//! tooling (planners, error messages, dialect hints). The two are
//! linked by name only; lowering converts an `OpDef` to a `BinOp` by
//! looking up the well-known constant tables.
//!
//! Same split for [`FuncDef`]: tree-side carrier of a function name +
//! arity + kind. There is no wire-side `FuncOp` analogue because
//! function calls are encoded with [`crate::expr::ops::OpFamily::FuncRef`]
//! plus a `FuncId` (interned in `dol-cas`).
//!
//! ## v2 conventions honoured
//!
//! - `Serialize`-only — no `Deserialize`. Wire input is the job of
//!   `dol-wire::Decode`. (No serde derive in this slice — `dol-ir`
//!   has no serde feature yet; one will be added when wire support
//!   arrives.)
//! - Names are stored as [`Name`] from `dol-core`, the canonical
//!   compact name type that supports both `&'static str` (zero-alloc)
//!   and `Box<str>` (runtime/custom).
//! - All public types are `#[non_exhaustive]` where the enum may grow
//!   in later phases.

extern crate alloc;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use core::fmt;

use dol_core::strings::Name;

// ─── OpCategory ──────────────────────────────────────────────────────

/// Classification of a DOL binary operator.
///
/// One-to-one with the high-level bands of [`crate::expr::ops::BinOp`]
/// (arithmetic 0–4, comparison 10–15, logical 20–21, bitwise 30–34,
/// concat 40, like 50–51). Categories that suggested tier confusion
/// (regex/glob, collection containment) live in the function
/// registry instead — they are not universally infix and so do not
/// qualify as [`OpDef`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// Regex and glob variants live in the function registry, not here.
    Pattern,
    /// String operators: `||` (concatenation).
    StringOp,
    /// Bitwise operators: `&`, `|`, `^`, `<<`, `>>`.
    Bitwise,
}

// ─── OpDef ───────────────────────────────────────────────────────────

/// Tree-side operator definition: `name` + `kind`.
///
/// Stored in the [`Binary`] variant of the (forthcoming) `Expr<'a>`
/// enum. Lowering converts an `OpDef` to a wire-side
/// [`crate::expr::ops::BinOp`] by looking up the well-known constant
/// tables exposed below.
///
/// Construct via [`OpDef::new_static`] for zero-allocation
/// well-known operators, or [`OpDef::custom`] for runtime / dialect-
/// specific ones.
///
/// [`Binary`]: dol-rewrite-plan-v2.md#82-tree-dsl
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OpDef {
    name: Name,
    kind: OpCategory,
}

impl OpDef {
    /// Construct from compile-time-known constants. Zero allocation.
    ///
    /// The `name` is stored as [`Name::Static`]; its lifetime is
    /// `'static`. Use this for operators with a registry constant.
    #[must_use]
    pub const fn new_static(name: &'static str, kind: OpCategory) -> Self {
        Self {
            name: Name::Static(name),
            kind,
        }
    }

    /// Construct a custom (runtime-named) operator definition.
    ///
    /// Allocates a `Box<str>` for the name. Use this only when the
    /// operator name is not known at compile time.
    pub fn custom(name: impl Into<Box<str>>, kind: OpCategory) -> Self {
        Self {
            name: Name::Owned(name.into()),
            kind,
        }
    }

    /// The operator name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// The operator category.
    #[inline]
    #[must_use]
    pub fn kind(&self) -> OpCategory {
        self.kind
    }
}

impl fmt::Display for OpDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl PartialEq<&str> for OpDef {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.name() == *other
    }
}

impl PartialEq<OpDef> for &str {
    #[inline]
    fn eq(&self, other: &OpDef) -> bool {
        *self == other.name()
    }
}

/// Well-known operator names, paired with their [`OpCategory`].
///
/// These names are stable wire-format identifiers. They map 1:1 to
/// the variants of [`crate::expr::ops::BinOp`] — see
/// [`OpDef::well_known`] below for the canonical tabulated form.
impl OpDef {
    // Comparison
    /// `=` — equality.
    pub const EQ: &'static str = "EQ";
    /// `!=` — inequality.
    pub const NE: &'static str = "NE";
    /// `<` — strictly less than.
    pub const LT: &'static str = "LT";
    /// `>` — strictly greater than.
    pub const GT: &'static str = "GT";
    /// `<=` — less than or equal.
    pub const LE: &'static str = "LE";
    /// `>=` — greater than or equal.
    pub const GE: &'static str = "GE";

    // Arithmetic
    /// `+` — addition.
    pub const ADD: &'static str = "ADD";
    /// `-` — subtraction.
    pub const SUB: &'static str = "SUB";
    /// `*` — multiplication.
    pub const MUL: &'static str = "MUL";
    /// `/` — division.
    pub const DIV: &'static str = "DIV";
    /// `%` — modulus.
    pub const MOD: &'static str = "MOD";

    // Logical
    /// `AND` — short-circuit conjunction.
    pub const AND: &'static str = "AND";
    /// `OR` — short-circuit disjunction.
    pub const OR: &'static str = "OR";

    // Bitwise
    /// `&` — bitwise AND.
    pub const BIT_AND: &'static str = "BIT_AND";
    /// `|` — bitwise OR.
    pub const BIT_OR: &'static str = "BIT_OR";
    /// `^` — bitwise XOR.
    pub const BIT_XOR: &'static str = "BIT_XOR";
    /// `<<` — bitwise shift left.
    pub const SHIFT_LEFT: &'static str = "SHIFT_LEFT";
    /// `>>` — bitwise shift right.
    pub const SHIFT_RIGHT: &'static str = "SHIFT_RIGHT";

    // String
    /// `||` — string concatenation.
    pub const CONCAT: &'static str = "CONCAT";

    // Pattern
    /// `LIKE` — SQL pattern match.
    pub const LIKE: &'static str = "LIKE";
    /// `ILIKE` — case-insensitive `LIKE`.
    pub const ILIKE: &'static str = "ILIKE";

    /// Tabulated catalogue of every well-known operator paired with
    /// its category. The order is stable and is locked by a snapshot
    /// test in this module — adding a new entry here is a wire-format
    /// change, not a casual edit.
    ///
    /// Length: matches the count of `pub const` operator names above.
    #[must_use]
    pub const fn well_known() -> &'static [(&'static str, OpCategory)] {
        &[
            // Comparison (6)
            (Self::EQ, OpCategory::Comparison),
            (Self::NE, OpCategory::Comparison),
            (Self::LT, OpCategory::Comparison),
            (Self::GT, OpCategory::Comparison),
            (Self::LE, OpCategory::Comparison),
            (Self::GE, OpCategory::Comparison),
            // Arithmetic (5)
            (Self::ADD, OpCategory::Arithmetic),
            (Self::SUB, OpCategory::Arithmetic),
            (Self::MUL, OpCategory::Arithmetic),
            (Self::DIV, OpCategory::Arithmetic),
            (Self::MOD, OpCategory::Arithmetic),
            // Logical (2)
            (Self::AND, OpCategory::Logical),
            (Self::OR, OpCategory::Logical),
            // Bitwise (5)
            (Self::BIT_AND, OpCategory::Bitwise),
            (Self::BIT_OR, OpCategory::Bitwise),
            (Self::BIT_XOR, OpCategory::Bitwise),
            (Self::SHIFT_LEFT, OpCategory::Bitwise),
            (Self::SHIFT_RIGHT, OpCategory::Bitwise),
            // String (1)
            (Self::CONCAT, OpCategory::StringOp),
            // Pattern (2)
            (Self::LIKE, OpCategory::Pattern),
            (Self::ILIKE, OpCategory::Pattern),
        ]
    }
}

// ─── Arity ───────────────────────────────────────────────────────────

/// Expected argument count for a DOL function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arity {
    /// Exactly `n` arguments required.
    Exact(u8),
    /// At least `n` arguments required (variadic tail).
    AtLeast(u8),
    /// Between `low` and `high` arguments inclusive.
    Range(u8, u8),
    /// Any number of arguments accepted (no validation).
    Any,
}

/// Error returned by [`FuncDef::validate_arity`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArityError {
    /// The name of the function that failed validation.
    pub func_name: String,
    /// The constraint declared for this function.
    pub expected: Arity,
    /// The number of arguments actually supplied.
    pub actual: usize,
}

impl fmt::Display for ArityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "function '{}': expected {} argument(s), got {}",
            self.func_name,
            match self.expected {
                Arity::Exact(n) => format!("exactly {n}"),
                Arity::AtLeast(n) => format!("at least {n}"),
                Arity::Range(low, high) => format!("{low}..={high}"),
                Arity::Any => "any number of".to_string(),
            },
            self.actual,
        )
    }
}

// ─── FuncKind ────────────────────────────────────────────────────────

/// Broad category of a DOL function.
///
/// Used by lowering and backends to pick the correct evaluation
/// strategy: scalar functions are evaluated row-by-row, aggregates
/// fold over a partition, window functions evaluate against a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FuncKind {
    /// A scalar function (e.g. `LOWER`, `ABS`, `ROUND`).
    Scalar,
    /// An aggregate function (e.g. `COUNT`, `SUM`, `AVG`).
    Aggregate,
    /// A window / ranking function (e.g. `ROW_NUMBER`, `RANK`).
    Window,
}

// ─── FuncDef ─────────────────────────────────────────────────────────

/// Tree-side function definition: `name` + `arity` + `kind`.
///
/// Stored in the `Call` variant of the (forthcoming) `Expr<'a>`
/// enum. The well-known function registry (`LENGTH`, `UPPER`,
/// `COUNT`, `SUM`, …) lands in M3c-β alongside the `Expr<'a>` enum;
/// this slice ships only the carrier type and `validate_arity`
/// helper so they can be reviewed in isolation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FuncDef {
    name: Name,
    arity: Arity,
    kind: FuncKind,
}

impl FuncDef {
    /// Construct from compile-time-known constants. Zero allocation.
    ///
    /// The `name` is stored as [`Name::Static`]; its lifetime is
    /// `'static`. Use this for functions with a registry entry.
    #[must_use]
    pub const fn new_static(name: &'static str, arity: Arity, kind: FuncKind) -> Self {
        Self {
            name: Name::Static(name),
            arity,
            kind,
        }
    }

    /// Construct a custom (runtime-named) scalar function with
    /// [`Arity::Any`].
    pub fn custom(name: impl Into<Box<str>>) -> Self {
        Self {
            name: Name::Owned(name.into()),
            arity: Arity::Any,
            kind: FuncKind::Scalar,
        }
    }

    /// Construct a custom function with explicit arity and kind.
    pub fn custom_with(name: impl Into<Box<str>>, arity: Arity, kind: FuncKind) -> Self {
        Self {
            name: Name::Owned(name.into()),
            arity,
            kind,
        }
    }

    /// The function name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// The arity constraint.
    #[inline]
    #[must_use]
    pub fn arity(&self) -> Arity {
        self.arity
    }

    /// The function category.
    #[inline]
    #[must_use]
    pub fn kind(&self) -> FuncKind {
        self.kind
    }

    /// Returns `Ok(())` when `arg_count` satisfies the arity
    /// constraint, or an [`ArityError`] otherwise.
    pub fn validate_arity(&self, arg_count: usize) -> Result<(), ArityError> {
        let ok = match self.arity {
            Arity::Exact(n) => arg_count == n as usize,
            Arity::AtLeast(n) => arg_count >= n as usize,
            Arity::Range(low, high) => arg_count >= low as usize && arg_count <= high as usize,
            Arity::Any => true,
        };
        if ok {
            Ok(())
        } else {
            Err(ArityError {
                func_name: self.name().to_string(),
                expected: self.arity,
                actual: arg_count,
            })
        }
    }
}

impl fmt::Display for FuncDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl PartialEq<&str> for FuncDef {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.name() == *other
    }
}

impl PartialEq<FuncDef> for &str {
    #[inline]
    fn eq(&self, other: &FuncDef) -> bool {
        *self == other.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── OpDef / OpCategory ──────────────────────────────────────────

    #[test]
    fn opdef_static_is_zero_alloc_and_round_trips() {
        let op = OpDef::new_static(OpDef::EQ, OpCategory::Comparison);
        assert_eq!(op.name(), "EQ");
        assert_eq!(op.kind(), OpCategory::Comparison);
        assert_eq!(op, "EQ");
        assert_eq!("EQ", op);
        assert_eq!(format!("{op}"), "EQ");
    }

    #[test]
    fn opdef_custom_uses_owned_name() {
        let op = OpDef::custom("MY_OP", OpCategory::Arithmetic);
        assert_eq!(op.name(), "MY_OP");
        assert_eq!(op.kind(), OpCategory::Arithmetic);
        // Equal to a static OpDef with the same name + category.
        let st = OpDef::new_static("MY_OP", OpCategory::Arithmetic);
        assert_eq!(op, st);
    }

    #[test]
    fn opdef_inequality_distinguishes_name_and_kind() {
        let a = OpDef::new_static(OpDef::EQ, OpCategory::Comparison);
        let b = OpDef::new_static(OpDef::NE, OpCategory::Comparison);
        let c = OpDef::new_static(OpDef::EQ, OpCategory::Logical); // bogus pairing
        assert_ne!(a, b);
        assert_ne!(a, c);
    }

    /// **Numeric-stability snapshot**: locks the well-known operator
    /// catalogue. Adding, removing, or reordering an entry is a
    /// wire-format change — bumping this test means the on-the-wire
    /// stable name set changed.
    #[test]
    fn well_known_catalogue_is_locked() {
        let wk = OpDef::well_known();
        assert_eq!(
            wk.len(),
            21,
            "well-known op catalogue size is part of the v2 wire format"
        );
        // Spot-check the boundaries of each band so a re-ordering of
        // the slice is also caught.
        assert_eq!(wk[0], (OpDef::EQ, OpCategory::Comparison));
        assert_eq!(wk[5], (OpDef::GE, OpCategory::Comparison));
        assert_eq!(wk[6], (OpDef::ADD, OpCategory::Arithmetic));
        assert_eq!(wk[10], (OpDef::MOD, OpCategory::Arithmetic));
        assert_eq!(wk[11], (OpDef::AND, OpCategory::Logical));
        assert_eq!(wk[12], (OpDef::OR, OpCategory::Logical));
        assert_eq!(wk[13], (OpDef::BIT_AND, OpCategory::Bitwise));
        assert_eq!(wk[17], (OpDef::SHIFT_RIGHT, OpCategory::Bitwise));
        assert_eq!(wk[18], (OpDef::CONCAT, OpCategory::StringOp));
        assert_eq!(wk[19], (OpDef::LIKE, OpCategory::Pattern));
        assert_eq!(wk[20], (OpDef::ILIKE, OpCategory::Pattern));
    }

    #[test]
    fn well_known_names_are_unique() {
        use alloc::collections::BTreeSet;
        let names: BTreeSet<&'static str> = OpDef::well_known().iter().map(|(n, _)| *n).collect();
        assert_eq!(names.len(), OpDef::well_known().len());
    }

    #[test]
    fn opdef_hash_is_value_equal() {
        use alloc::collections::BTreeMap;
        // Hash is content-based: Static and Owned representations of
        // the same name compare equal in the right places.
        let st = OpDef::new_static("XYZ", OpCategory::Logical);
        let ow = OpDef::custom("XYZ", OpCategory::Logical);
        let mut m = BTreeMap::new();
        m.insert(format!("{st}"), 1);
        assert_eq!(m.get(&format!("{ow}")), Some(&1));
    }

    // ─── Arity / FuncDef ─────────────────────────────────────────────

    #[test]
    fn funcdef_static_is_zero_alloc_and_round_trips() {
        const COUNT: FuncDef = FuncDef::new_static("COUNT", Arity::Exact(1), FuncKind::Aggregate);
        assert_eq!(COUNT.name(), "COUNT");
        assert_eq!(COUNT.arity(), Arity::Exact(1));
        assert_eq!(COUNT.kind(), FuncKind::Aggregate);
        assert_eq!(COUNT, "COUNT");
        assert_eq!("COUNT", COUNT);
        assert_eq!(format!("{COUNT}"), "COUNT");
    }

    #[test]
    fn funcdef_custom_defaults_to_scalar_any() {
        let f = FuncDef::custom("MY_FUNC");
        assert_eq!(f.name(), "MY_FUNC");
        assert_eq!(f.arity(), Arity::Any);
        assert_eq!(f.kind(), FuncKind::Scalar);
    }

    #[test]
    fn funcdef_custom_with_overrides() {
        let f = FuncDef::custom_with("RANK", Arity::Exact(0), FuncKind::Window);
        assert_eq!(f.arity(), Arity::Exact(0));
        assert_eq!(f.kind(), FuncKind::Window);
    }

    #[test]
    fn validate_arity_exact() {
        let f = FuncDef::new_static("LOWER", Arity::Exact(1), FuncKind::Scalar);
        assert!(f.validate_arity(1).is_ok());
        let err = f.validate_arity(0).unwrap_err();
        assert_eq!(err.actual, 0);
        assert_eq!(err.expected, Arity::Exact(1));
        assert_eq!(err.func_name, "LOWER");
        // Display includes the name, the constraint, and the actual.
        let msg = format!("{err}");
        assert!(msg.contains("LOWER"), "{msg}");
        assert!(msg.contains("exactly 1"), "{msg}");
        assert!(msg.contains("got 0"), "{msg}");
    }

    #[test]
    fn validate_arity_at_least() {
        let f = FuncDef::new_static("CONCAT", Arity::AtLeast(2), FuncKind::Scalar);
        assert!(f.validate_arity(2).is_ok());
        assert!(f.validate_arity(7).is_ok());
        let err = f.validate_arity(1).unwrap_err();
        assert!(format!("{err}").contains("at least 2"));
    }

    #[test]
    fn validate_arity_range() {
        let f = FuncDef::new_static("ROUND", Arity::Range(1, 2), FuncKind::Scalar);
        assert!(f.validate_arity(1).is_ok());
        assert!(f.validate_arity(2).is_ok());
        assert!(f.validate_arity(0).is_err());
        let err = f.validate_arity(3).unwrap_err();
        assert!(format!("{err}").contains("1..=2"));
    }

    #[test]
    fn validate_arity_any_accepts_everything() {
        let f = FuncDef::new_static("DEBUG", Arity::Any, FuncKind::Scalar);
        for n in 0..256 {
            assert!(f.validate_arity(n).is_ok());
        }
    }

    #[test]
    fn funcdef_inequality_distinguishes_all_three_fields() {
        let base = FuncDef::new_static("F", Arity::Exact(1), FuncKind::Scalar);
        let diff_name = FuncDef::new_static("G", Arity::Exact(1), FuncKind::Scalar);
        let diff_arity = FuncDef::new_static("F", Arity::Exact(2), FuncKind::Scalar);
        let diff_kind = FuncDef::new_static("F", Arity::Exact(1), FuncKind::Aggregate);
        assert_ne!(base, diff_name);
        assert_ne!(base, diff_arity);
        assert_ne!(base, diff_kind);
    }

    #[test]
    fn funckind_variants_are_distinct() {
        // Sanity check the three categories don't accidentally
        // collapse via Hash/Eq.
        use alloc::collections::BTreeSet;
        let mut s = BTreeSet::new();
        s.insert(format!("{:?}", FuncKind::Scalar));
        s.insert(format!("{:?}", FuncKind::Aggregate));
        s.insert(format!("{:?}", FuncKind::Window));
        assert_eq!(s.len(), 3);
    }
}
