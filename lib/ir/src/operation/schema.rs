//! Schema-level structural operations (`Operation::Schema`).
//!
//! `SchemaOp` covers create/alter/drop/rename/truncate of an entity-level
//! object: a relation, document collection, KV bucket, blob bucket, file
//! tree, API resource, stream topic, virtual view, or a named type.
//!
//! Schemas are referenced through the program's catalog
//! ([`SchemaRef`](crate::schema_ref::SchemaRef)) rather than embedded
//! inline. Backends that need the full schema body resolve it through
//! [`SchemaCatalog`](crate::schema_catalog::SchemaCatalog).

use crate::schema_ref::SchemaRef;
use crate::target::{Symbol, Target};

/// The verb applied by a structural / governance operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StructuralVerb {
    /// Create the object if it does not exist.
    Create,
    /// Drop the object.
    Drop,
    /// Alter the object's body.
    Alter,
    /// Rename the object.
    Rename,
    /// Empty the object's contents (data-bearing targets only).
    Truncate,
}

/// Body kind of a [`SchemaOp`] payload.
///
/// `Entity` covers relations, documents, KV buckets, blobs, file trees,
/// streams, and APIs — anything that has fields/columns/keys.
/// `Type` covers named types (enums, composite types, distinct types).
/// `Reference` carries no body and is the canonical shape for `Drop`,
/// `Rename`, and `Truncate`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SchemaBody {
    /// Entity body referenced by catalog id.
    Entity { schema: SchemaRef, if_not_exists: bool },
    /// Named type body referenced by catalog id.
    Type { schema: SchemaRef, body: TypeBody },
    /// No body — used for `Drop` / `Rename` / `Truncate`.
    Reference,
}

/// Type-body classification for `Schema { verb: Create, body: Type, .. }`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TypeBody {
    /// Enumerated type.
    Enum,
    /// Composite / record type.
    Composite,
    /// Distinct (domain) type.
    Distinct,
    /// Backend-specific kind.
    Other,
}

/// Schema-level structural operation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchemaOp {
    pub verb: StructuralVerb,
    pub target: Target,
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
