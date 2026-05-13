//! Runtime [`Value`] enum and its bound-pair [`ValueRange`].
//!
//! `Value` is the owned runtime form: the output of decoding, the input to
//! binding, and the unit of storage and transport. `ValueRange` is its
//! companion bound pair (used for `Value::Range`).
//!
//! See [`crate::literal`] for the borrowed AST counterpart `Literal<'a>`
//! (with `Cow`-backed strings/bytes for zero-copy parsing) and the
//! `Literal ↔ Value` conversions.
//!
//! # Module layout
//!
//! - `enum_def` — the [`Value`] enum.
//! - `range` — [`ValueRange`].
//! - `classify` — `is_*` predicates and [`Value::type_name`].
//! - `accessors` — typed `as_*` helpers.
//! - `display` — `impl fmt::Display for Value`.
//! - `from_impls` — `From<…>` conversions for primitive payloads.

mod accessors;
mod classify;
mod display;
mod enum_def;
mod from_impls;
mod range;

#[cfg(test)]
mod tests;

pub use enum_def::Value;
pub use range::ValueRange;
pub(crate) use range::fmt_bound;

// Backwards-compatibility re-export: `dol_core::value::Literal` / `LiteralRange`
// previously lived inside this module. They moved to [`crate::literal`] but the
// old paths remain valid.
#[doc(hidden)]
pub use crate::literal::{Literal, LiteralRange};
