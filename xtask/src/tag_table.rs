//! Tag-table drift gate for the operator/expression tag tables.
//!
//! The four-tier rule (see `lib/expr/src/expr.rs`) freezes three
//! tag tables for the v2 wire format:
//!
//! * [`dol_expr::BinOp`] — `as_u16` / `try_from_u16`
//! * [`dol_expr::UnaryOp`] — `as_u16` / `try_from_u16`
//! * [`dol_expr::ExprOp`] — `as u8` / `from_u8`
//!
//! Tags must never be renumbered; new variants must use the next free
//! tag and add a row to the corresponding `EXPECTED_*` snapshot below.
//! This module asserts that the live enum and the snapshot agree:
//!
//! 1. Each `(name, tag)` pair in the snapshot resolves to the named
//!    variant via the public `try_from_*` / `from_u8` constructor.
//! 2. The first tag past the snapshot returns `None`, ensuring an
//!    accidental enum addition without a snapshot update is rejected.
//!
//! On drift, the gate prints which table changed and which tag is
//! out of sync. The fix is to update *both* the enum and this snapshot
//! in the same PR.

use dol_expr::{BinOp, ExprOp, UnaryOp};

/// Frozen `(variant_name, tag)` snapshot for [`BinOp`].
///
/// Append-only: never renumber an existing entry; never reuse a tag
/// previously assigned to a removed variant within the same major
/// version.
pub const EXPECTED_BINOP: &[(&str, u16)] = &[
    ("Eq", 0),
    ("Ne", 1),
    ("Lt", 2),
    ("Le", 3),
    ("Gt", 4),
    ("Ge", 5),
    ("And", 6),
    ("Or", 7),
    ("Add", 8),
    ("Sub", 9),
    ("Mul", 10),
    ("Div", 11),
    ("Rem", 12),
    ("Like", 13),
    ("ILike", 14),
    ("Similar", 15),
    ("BitAnd", 16),
    ("BitOr", 17),
    ("BitXor", 18),
    ("Shl", 19),
    ("Shr", 20),
    ("Concat", 21),
    ("IsDistinctFrom", 22),
    ("IsNotDistinctFrom", 23),
];

/// Frozen `(variant_name, tag)` snapshot for [`UnaryOp`].
///
/// Append-only: never renumber an existing entry; never reuse a tag
/// previously assigned to a removed variant within the same major
/// version. (Same contract as [`EXPECTED_BINOP`].)
pub const EXPECTED_UNARY: &[(&str, u16)] = &[
    ("Neg", 0),
    ("Not", 1),
    ("BitNot", 2),
    ("IsNull", 3),
    ("IsNotNull", 4),
    ("IsTrue", 5),
    ("IsFalse", 6),
];

/// Frozen `(variant_name, tag)` snapshot for [`ExprOp`].
///
/// Append-only: never renumber an existing entry; never reuse a tag
/// previously assigned to a removed variant within the same major
/// version. (Same contract as [`EXPECTED_BINOP`].)
pub const EXPECTED_EXPROP: &[(&str, u8)] = &[
    ("Nop", 0),
    ("Namespace", 1),
    ("Field", 2),
    ("Param", 3),
    ("Lit", 4),
    ("Composite", 5),
    ("Bin", 6),
    ("Una", 7),
    ("Func", 8),
    ("Agg", 9),
    ("Window", 10),
    ("Cast", 11),
    ("Case", 12),
    ("Alias", 13),
    ("In", 14),
    ("Exists", 15),
    ("Between", 16),
    ("Query", 17),
    ("Insert", 18),
    ("Update", 19),
    ("Delete", 20),
    ("Upsert", 21),
];

/// Resolve a `BinOp` snapshot row to its live variant name.
///
/// Returns the snapshot name on success (because every variant has a
/// stable spelling we agreed to in the table), or an explanatory
/// `Err(String)` describing the drift.
fn check_binop(name: &str, tag: u16) -> Result<(), String> {
    let live = BinOp::try_from_u16(tag)
        .ok_or_else(|| format!("BinOp::try_from_u16({tag}) returned None (expected `{name}`)"))?;
    let live_name = binop_variant_name(live);
    if live_name != name {
        return Err(format!(
            "BinOp tag {tag} is `{live_name}` in the enum but `{name}` in the snapshot"
        ));
    }
    Ok(())
}

fn check_unary(name: &str, tag: u16) -> Result<(), String> {
    let live = UnaryOp::try_from_u16(tag).ok_or_else(|| {
        format!("UnaryOp::try_from_u16({tag}) returned None (expected `{name}`)")
    })?;
    let live_name = unary_variant_name(live);
    if live_name != name {
        return Err(format!(
            "UnaryOp tag {tag} is `{live_name}` in the enum but `{name}` in the snapshot"
        ));
    }
    Ok(())
}

fn check_exprop(name: &str, tag: u8) -> Result<(), String> {
    let live = ExprOp::from_u8(tag)
        .ok_or_else(|| format!("ExprOp::from_u8({tag}) returned None (expected `{name}`)"))?;
    let live_name = exprop_variant_name(live);
    if live_name != name {
        return Err(format!(
            "ExprOp tag {tag} is `{live_name}` in the enum but `{name}` in the snapshot"
        ));
    }
    Ok(())
}

/// Map a live `BinOp` to its source-spelt variant name.
///
/// The `Debug` impl for `BinOp` is `derive`d, so it prints the variant
/// name verbatim. We use `format!` rather than a hand-rolled match so
/// that adding a new variant doesn't require also updating this helper.
fn binop_variant_name(op: BinOp) -> String {
    format!("{op:?}")
}
fn unary_variant_name(op: UnaryOp) -> String {
    format!("{op:?}")
}
fn exprop_variant_name(op: ExprOp) -> String {
    format!("{op:?}")
}

/// Run the gate. Returns `Ok(())` on success, `Err(message)` describing
/// the first detected drift on failure.
pub fn run() -> Result<(), String> {
    // Validate every recorded row.
    for (name, tag) in EXPECTED_BINOP {
        check_binop(name, *tag)?;
    }
    for (name, tag) in EXPECTED_UNARY {
        check_unary(name, *tag)?;
    }
    for (name, tag) in EXPECTED_EXPROP {
        check_exprop(name, *tag)?;
    }

    // The next tag past the last snapshot row must NOT decode — that
    // catches accidental enum extensions without a corresponding
    // snapshot update.
    let next_binop = EXPECTED_BINOP.last().map_or(0u16, |(_, t)| t + 1);
    if BinOp::try_from_u16(next_binop).is_some() {
        return Err(format!(
            "BinOp gained a new variant at tag {next_binop} but the EXPECTED_BINOP snapshot \
             in `xtask/src/tag_table.rs` was not updated"
        ));
    }
    let next_unary = EXPECTED_UNARY.last().map_or(0u16, |(_, t)| t + 1);
    if UnaryOp::try_from_u16(next_unary).is_some() {
        return Err(format!(
            "UnaryOp gained a new variant at tag {next_unary} but the EXPECTED_UNARY snapshot \
             in `xtask/src/tag_table.rs` was not updated"
        ));
    }
    let next_exprop = EXPECTED_EXPROP.last().map_or(0u8, |(_, t)| t + 1);
    if ExprOp::from_u8(next_exprop).is_some() {
        return Err(format!(
            "ExprOp gained a new variant at tag {next_exprop} but the EXPECTED_EXPROP snapshot \
             in `xtask/src/tag_table.rs` was not updated"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Snapshot agrees with the enum (the gate's positive-path
    /// assertion). If this fails, the snapshot must be updated.
    #[test]
    fn snapshot_matches_live_enum() {
        run().expect("tag-table snapshot drift");
    }

    /// `check_binop` reports drift cleanly. Use a known-good entry.
    #[test]
    fn binop_check_accepts_known_pair() {
        assert!(check_binop("Eq", 0).is_ok());
    }

    /// `check_binop` rejects a swapped name/tag pair.
    #[test]
    fn binop_check_rejects_swapped_pair() {
        // `Eq` is at tag 0, not 1.
        let err = check_binop("Eq", 1).unwrap_err();
        assert!(err.contains("Eq"));
    }
}
