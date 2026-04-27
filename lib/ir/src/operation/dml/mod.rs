//! Data-manipulation (DML) operation payloads.
//!
//! Verb-shaped operations that change rows / documents / objects / files:
//! `Insert`, `Update`, `Replace`, `Delete`, `Upsert`, `Append`.

pub mod append;
pub mod delete;
pub mod insert;
pub mod replace;
pub mod update;
pub mod upsert;

pub use append::Append;
pub use delete::Delete;
pub use insert::{Insert, InsertSource};
pub use replace::{Replace, ReplaceBody};
pub use update::Update;
pub use upsert::Upsert;
