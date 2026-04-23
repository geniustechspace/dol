//! DOL fluent builder API — produces IR statements (internal).

#![deny(unsafe_code)]

pub mod mutate;
pub mod select;

pub use mutate::{DeleteBuilder, InsertBuilder, UpdateBuilder};
pub use select::SelectBuilder;
