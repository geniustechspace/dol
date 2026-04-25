//! # `dol-arena` — generic arena + interner + typed ids
//!
//! Pure infrastructure used by every DOL layer that allocates AST/IR nodes.
//! This crate has **no DOL semantics**: it knows nothing about expressions,
//! statements, schemas, or wire formats. Higher layers parameterise its
//! generics with their own marker types.
//!
//! ## Design
//!
//! - [`Arena<T>`] is a contiguous `Vec<T>` keyed by [`Id<T, Tag>`]. Ids are
//!   `u32` indices; they cannot be confused across arenas because the phantom
//!   `Tag` type tags each id.
//! - [`Interner`] gives every unique byte sequence a stable [`StrId`]. Backed
//!   by `hashbrown` with the default hasher, `Arc<str>` storage, and a single
//!   shared allocation per unique string.
//! - [`StrId`] is a newtype `u32` with a 24-bit index + 8-bit user-defined
//!   `kind` byte for cheap categorisation (e.g. "identifier" vs. "literal").
//!
//! ## Sizes
//!
//! Asserted in `tests`:
//!
//! ```text
//! size_of::<Id<u8, ()>>()  == 4
//! size_of::<StrId>()       == 4
//! ```
//!
//! ## Features
//!
//! | feature  | effect                                               |
//! |----------|------------------------------------------------------|
//! | `serde`  | `Serialize`/`Deserialize` for `Id`, `StrId`, arenas. |
//! | `std`    | implements `std::error::Error` for arena errors.     |
//!
//! Default build is `no_std + alloc`.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

mod arena;
mod id;
mod interner;

pub use arena::Arena;
pub use id::{Id, NULL_ID, StrId};
pub use interner::Interner;
