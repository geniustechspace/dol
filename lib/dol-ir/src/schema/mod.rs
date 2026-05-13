//! Schema types — `Entity`, `Field`, `Relation`, `Lookup`, `Policy`, `Constraint`.
//!
//! Per `dol-rewrite-plan-v2.md` §8.6. This is the **M3d-α** skeleton:
//! type definitions only. `SchemaCatalog` and validation passes land
//! in M3d-β.
//!
//! All handle types (`EntityId`, `FieldId`, `RelationId`, `LookupId`,
//! `PolicyId`, `NodeId`, `StrId`) come from [`dol_cas::handle`].

pub mod constraint;
pub mod entity;
pub mod field;
pub mod lookup;
pub mod policy;
pub mod relation;

pub use constraint::{Constraint, ConstraintKind};
pub use entity::Entity;
pub use field::{Field, FieldType};
pub use lookup::Lookup;
pub use policy::{Policy, PolicyKind};
pub use relation::{Relation, RelationKind};
