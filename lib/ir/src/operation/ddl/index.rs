//! Index structural operations (`Operation::Index`).

use dol_expr::ids::NodeId;
use smallvec::SmallVec;

use crate::operation::shared::StructuralVerb;
use crate::target::{Symbol, Target};

/// Sort direction for an index column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IndexDirection {
    Ascending,
    Descending,
}

/// One column / expression in an index key.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IndexKey {
    /// Plain field key.
    Field {
        name: Symbol,
        direction: IndexDirection,
    },
    /// Arena expression key (functional index).
    Expression {
        node: NodeId,
        direction: IndexDirection,
    },
}

/// Method / kind of an index. This is open-ended via `Custom` so backends
/// can carry their own tags without a closed enum extension.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IndexMethod {
    BTree,
    Hash,
    Gin,
    Gist,
    Vector,
    Spatial,
    Custom(Symbol),
}

/// Index structural operation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IndexOp {
    pub verb: StructuralVerb,
    pub target: Target,
    pub name: Symbol,
    pub method: IndexMethod,
    pub keys: SmallVec<[IndexKey; 2]>,
    pub unique: bool,
    /// Arena `NodeId` for a partial-index predicate.
    pub predicate: Option<NodeId>,
}
