//! Index structural operations (`Operation::Index`).

use dol_expr::ids::NodeId;
use smallvec::SmallVec;

use crate::operation::shared::StructuralVerb;
use crate::target::{Symbol, Target};

/// Sort direction for an index column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum IndexDirection {
    /// Sort values in ascending order (smallest first).
    Ascending,
    /// Sort values in descending order (largest first).
    Descending,
}

/// One column / expression in an index key.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum IndexKey {
    /// Plain field key.
    Field {
        /// Interned field name.
        name: Symbol,
        /// Sort direction for this key component.
        direction: IndexDirection,
    },
    /// Arena expression key (functional index).
    Expression {
        /// Arena `NodeId` of the index expression.
        node: NodeId,
        /// Sort direction for this key component.
        direction: IndexDirection,
    },
}

/// Method / kind of an index. This is open-ended via `Custom` so backends
/// can carry their own tags without a closed enum extension.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum IndexMethod {
    /// B-tree index (default for most SQL databases).
    BTree,
    /// Hash index for equality-only lookups.
    Hash,
    /// Generalized inverted index (PostgreSQL GIN).
    Gin,
    /// Generalized search tree (PostgreSQL GiST).
    Gist,
    /// Vector-similarity index (pgvector, Pinecone).
    Vector,
    /// Spatial / geospatial index (PostGIS, Mongo 2dsphere).
    Spatial,
    /// Backend-specific index method identified by interned name.
    Custom(Symbol),
}

/// Index structural operation.
///
/// Maps to SQL `CREATE INDEX` / `DROP INDEX` / `ALTER INDEX`, or to a
/// document-store index definition. The [`Self::verb`] picks the lifecycle
/// action.
///
/// # Examples
///
/// ```
/// use dol_ir::operation::{IndexDirection, IndexKey, IndexMethod, IndexOp, StructuralVerb};
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::Operation;
///
/// // CREATE INDEX idx_users_email ON users (email ASC)
/// let op: Operation = IndexOp {
///     verb: StructuralVerb::Create,
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(0))),
///     name: Symbol::from_hash(1),
///     method: IndexMethod::BTree,
///     keys: smallvec::smallvec![IndexKey::Field {
///         name: Symbol::from_hash(2),
///         direction: IndexDirection::Ascending,
///     }],
///     unique: false,
///     predicate: None,
///     if_not_exists: false,
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::OpKind::Index);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IndexOp {
    /// `Create` / `Drop` / `Alter` / `Rename`.
    pub verb: StructuralVerb,
    /// Target the index is defined on.
    pub target: Target,
    /// Interned index name.
    pub name: Symbol,
    /// Index method (BTree, Hash, Vector, …).
    pub method: IndexMethod,
    /// Ordered list of index key columns / expressions.
    pub keys: SmallVec<[IndexKey; 2]>,
    /// Whether the index enforces uniqueness.
    pub unique: bool,
    /// Arena `NodeId` for a partial-index predicate.
    pub predicate: Option<NodeId>,
    /// Idempotency hint. For `Create`, suppress the failure when the index
    /// already exists (`CREATE INDEX IF NOT EXISTS`). For `Drop`, suppress
    /// the failure when the index is missing (`DROP INDEX IF EXISTS`).
    /// Backends that don't natively support the flag should treat it as a
    /// best-effort hint and apply their own pre-check.
    pub if_not_exists: bool,
}
