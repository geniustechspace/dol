//! # `string` — canonical interned UTF-8 strings.
//!
//! This module owns DOL's canonical string interner and its id type.
//!
//! Textual names such as fields, paths, functions, aliases, and type names
//! should be stored once in [`Interner`] and referenced by [`StrId`] where a
//! compact, deterministic identity is needed.
//!
//! The interner is deliberately string-specific. It stores UTF-8 text only;
//! non-string payloads should live in their own arenas or pools.

mod ids;
mod interner;
mod name;

pub use ids::{StrId, StrTag};
pub use interner::{InternError, Interner};
pub use name::Name;
