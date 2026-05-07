//! Data-definition (DDL) operation payloads.
//!
//! Structural / catalog-shape changes: `Schema` (entity), `Field`, `Index`,
//! `Lookup`. Each payload carries a [`StructuralVerb`](super::shared::StructuralVerb).

pub mod field;
pub mod index;
pub mod lookup;
pub mod schema;

pub use field::{FieldDef, FieldOp};
pub use index::{IndexDirection, IndexKey, IndexMethod, IndexOp};
pub use lookup::{LookupMethod, LookupOp};
pub use schema::{SchemaBody, SchemaOp};

// Re-export TypeBody from dol_schema (canonical home)
pub use dol_schema::TypeBody;
