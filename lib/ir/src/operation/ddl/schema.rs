//! Schema-level structural operations (`Operation::Schema`).
//!
//! `SchemaOp` covers create/alter/drop/rename/truncate of an entity-level
//! object: a relation, document collection, KV bucket, blob bucket, file
//! tree, API resource, stream topic, virtual view, or a named type.
//!
//! Schemas are referenced through the program's catalog
//! ([`SchemaRef`]) rather than embedded inline. Backends that need the full
//! schema body resolve it through
//! [`SchemaCatalog`](dol_schema::SchemaCatalog).

use crate::operation::shared::StructuralVerb;
use crate::target::{Symbol, Target};

// Import from dol_schema (now canonical home)
use dol_schema::{SchemaRef, TypeBody};

/// Body kind of a [`SchemaOp`] payload.
///
/// `Entity` covers relations, documents, KV buckets, blobs, file trees,
/// streams, and APIs — anything that has fields/columns/keys.
/// `Type` covers named types (enums, composite types, distinct types).
/// `Reference` carries no body and is the canonical shape for `Drop`,
/// `Rename`, and `Truncate`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum SchemaBody {
    /// Entity body referenced by catalog id.
    Entity {
        /// Catalog reference to the entity schema.
        schema: SchemaRef,
        /// When `true`, the operation is idempotent (`IF NOT EXISTS`).
        if_not_exists: bool,
    },
    /// Named type body referenced by catalog id.
    Type {
        /// Catalog reference to the type schema.
        schema: SchemaRef,
        /// Classification of the type (enum, composite, distinct).
        body: TypeBody,
    },
    /// No body — used for `Drop` / `Rename` / `Truncate`.
    Reference,
}

/// Schema-level structural operation.
///
/// Maps to SQL `CREATE TABLE` / `DROP TABLE` / `ALTER TABLE` / `TRUNCATE`,
/// or to document-store collection lifecycle, blob bucket creation, etc.
///
/// # Examples
///
/// ```
/// use dol_ir::operation::{SchemaBody, SchemaOp, StructuralVerb};
/// use dol_schema::{CatalogId, SchemaId, SchemaRef};
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::operation::Operation;
///
/// // CREATE TABLE users (...)
/// let op: Operation = SchemaOp {
///     verb: StructuralVerb::Create,
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(0))),
///     body: SchemaBody::Entity {
///         schema: SchemaRef::local(SchemaId::new(0)),
///         if_not_exists: false,
///     },
///     new_name: None,
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::operation::OpKind::Schema);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SchemaOp {
    /// `Create` / `Drop` / `Alter` / `Rename` / `Truncate`.
    pub verb: StructuralVerb,
    /// Target being created / dropped / altered.
    pub target: Target,
    /// Body of the schema (entity, type, or reference-only).
    pub body: SchemaBody,
    /// Optional new name when `verb == Rename`.
    pub new_name: Option<Symbol>,
}

impl SchemaOp {
    /// Build a `Create`-shaped entity schema operation.
    pub fn create_entity(target: Target, schema: SchemaRef, if_not_exists: bool) -> Self {
        Self {
            verb: StructuralVerb::Create,
            target,
            body: SchemaBody::Entity {
                schema,
                if_not_exists,
            },
            new_name: None,
        }
    }

    /// Build a `Drop`-shaped operation.
    pub fn drop_(target: Target) -> Self {
        Self {
            verb: StructuralVerb::Drop,
            target,
            body: SchemaBody::Reference,
            new_name: None,
        }
    }

    /// Build a `Rename` operation.
    pub fn rename(target: Target, new_name: Symbol) -> Self {
        Self {
            verb: StructuralVerb::Rename,
            target,
            body: SchemaBody::Reference,
            new_name: Some(new_name),
        }
    }

    /// Build a `Truncate` operation. Capability checks are responsible for
    /// rejecting this on non-data-bearing target kinds (per RFC §1).
    pub fn truncate(target: Target) -> Self {
        Self {
            verb: StructuralVerb::Truncate,
            target,
            body: SchemaBody::Reference,
            new_name: None,
        }
    }
}
