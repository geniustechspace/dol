//! Data-query (DQL) operation payloads.
//!
//! Read-only verbs: `Query` (the universal read), `Probe` (existence /
//! cheap membership), `Describe` (schema introspection).

pub mod describe;
pub mod probe;
pub mod query;

pub use describe::{Describe, DescribeFacet};
pub use probe::Probe;
pub use query::Query;
