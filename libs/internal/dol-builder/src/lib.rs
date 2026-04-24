//! DOL fluent builder API — produces IR statements (internal).

#![deny(unsafe_code)]

pub mod mutate;
pub mod query;

pub use mutate::{DeleteBuilder, InsertBuilder, UpdateBuilder, UpsertBuilder};
pub use query::QueryBuilder;
