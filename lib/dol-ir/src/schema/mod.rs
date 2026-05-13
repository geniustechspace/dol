//! Schema types — `Entity`, `Field`, `Relation`, `Lookup`, `Policy`,
//! `Constraint`, and the [`SchemaCatalog`] container that owns them.
//!
//! Per `dol-rewrite-plan-v2.md` §8.6. The vocabulary is deliberately
//! **backend-neutral** — DOL is a data-operating language, not SQL —
//! so the same shapes describe document stores, graph databases,
//! time-series buckets, and relational backends without privileging
//! any one of them.
//!
//! - **M3d-α** (shipped): record type skeletons.
//! - **M3d-β** (this slice): [`SchemaCatalog`] container plus
//!   `define_*` / `*_by_name` helpers and the [`CatalogError`] enum.
//! - **M3d-γ** (pending): constraint / FK / lookup integrity
//!   validation passes.
//!
//! All handle types (`EntityId`, `FieldId`, `RelationId`, `LookupId`,
//! `PolicyId`, `NodeId`, `StrId`) come from [`dol_cas::handle`].

pub mod catalog;
pub mod constraint;
pub mod entity;
pub mod field;
pub mod lookup;
pub mod policy;
pub mod relation;

pub use catalog::{CatalogError, SchemaCatalog};
pub use constraint::{Constraint, ConstraintKind};
pub use entity::Entity;
pub use field::{Field, FieldType};
pub use lookup::Lookup;
pub use policy::{Policy, PolicyKind};
pub use relation::{Relation, RelationKind};
