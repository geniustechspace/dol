//! Compatibility layer for v1 callers.
//!
//! `dol-ir` v2 keeps the v1 [`Statement`](crate::Statement) surface available
//! through this module while in-tree consumers migrate. The conversions in
//! [`statement`] are the canonical mapping documented in
//! `docs/rfcs/0001-ir-v2.md` (the "Mapping Old → New" table).

pub mod statement;
