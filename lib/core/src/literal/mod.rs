//! Borrowed-or-owned literal AST nodes.
//!
//! [`Literal<'a>`] is the lifetime-parameterized counterpart to
//! [`crate::Value`]: optimised for zero-copy AST construction (`Cow`-backed
//! strings/bytes; inline scalars; heap-boxed composites). Use [`Value`] for
//! runtime data, `Literal` for parser/AST output.
//!
//! See the [crate-level documentation][crate] for the full type inventory
//! and design rationale.
//!
//! # Module layout
//!
//! - `enum_def` — the [`Literal`] enum.
//! - `range` — [`LiteralRange`].
//! - `constructors` — `null`/`bool`/`string_borrowed`/… plus `is_*` and `as_*`.
//! - `display` — `impl fmt::Display for Literal<'a>`.
//! - `conversions` — `Literal → Value` (via `From`/`into_owned`),
//!   `Literal::into_static`, and `From<X> for Literal<'_>`.
//!
//! [`Value`]: crate::Value

mod constructors;
mod conversions;
mod display;
mod enum_def;
mod range;

#[cfg(test)]
mod tests;

pub use enum_def::Literal;
pub use range::LiteralRange;
