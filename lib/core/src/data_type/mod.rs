//! Type descriptors: what shape and constraints a field or parameter holds.
//!
//! [`DataType`] is the type-descriptor counterpart to [`crate::Value`]. It
//! answers **"what type is expected here?"** where `Value` answers
//! **"what data is actually here?"**.
//!
//! # Design
//!
//! - [`DataType`] mirrors every [`crate::Value`] variant but adds constraints
//!   (e.g., `String { max_len, fixed }` adds a max-length and a fixed/variable
//!   flag).
//! - [`StructField`] is a named, typed field used inside `DataType::Struct`.
//! - [`DataType::accepts`] is the conformance bridge: it validates that a
//!   runtime [`crate::Value`] satisfies the declared type.
//!
//! # Nullability
//!
//! Nullability is NOT embedded inside `DataType` (e.g., no
//! `DataType::Nullable` variant). It is a property of the *position* — the
//! field — not of the type itself. Embedding nullability would make
//! `DataType::Nullable(DataType::Nullable(...))` representable but meaningless.
//! Nullability belongs on the field definition (e.g., `StructField::nullable`).
//!
//! # No `Display`
//!
//! `DataType` deliberately does **not** implement [`core::fmt::Display`].
//! Backend-specific spellings (e.g. `VARCHAR(255)`, `String`, `text`) live
//! in the corresponding backend crate. For human-readable diagnostics, use
//! the auto-derived [`Debug`] impl or [`DataType::type_name`].
//!
//! # Module layout
//!
//! - `enum_def` — the [`DataType`] enum itself.
//! - `struct_field` — the [`StructField`] named-field record.
//! - `classify` — `is_*` predicates and [`DataType::type_name`].
//! - `conformance` — [`DataType::accepts`] (the largest impl).
//! - `constructors` — convenience constructors (`varying_string`, …).

mod classify;
mod conformance;
mod constructors;
mod enum_def;
mod struct_field;

#[cfg(test)]
mod tests;

pub use enum_def::DataType;
pub use struct_field::StructField;
