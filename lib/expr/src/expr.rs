use smallvec::SmallVec;

use crate::ids::{
    CaseId, DeleteId, FieldId, FuncId, InListId, InsertId, LiteralId, NodeId, ObjLitId, QueryId,
    StrId, UpdateId, UpsertId, WindowId,
};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum BinOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Like,
    ILike,
    Similar,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Concat,
    Arrow,
    LongArrow,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum UnaryOp {
    Neg,
    Not,
    /// Bitwise NOT (one's complement) on integer-typed operands. Distinct
    /// from `Not` (logical) so backends and lowerers cannot conflate the
    /// two on integer expressions.
    BitNot,
    IsNull,
    IsNotNull,
    IsTrue,
    IsFalse,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Order {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum LockHint {
    ForUpdate,
    ForShare,
    SkipLocked,
    NoWait,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum ConflictClause {
    DoNothing,
    DoUpdate {
        assignments: SmallVec<[(StrId, NodeId); 4]>,
    },
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct JoinNode {
    pub source: StrId,
    pub alias: Option<StrId>,
    pub join_type: JoinType,
    pub on: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QueryNode {
    pub from: StrId,
    pub alias: Option<StrId>,
    pub joins: SmallVec<[JoinNode; 2]>,
    pub filter: NodeId,
    pub columns: SmallVec<[NodeId; 8]>,
    pub group_by: SmallVec<[NodeId; 4]>,
    pub having: NodeId,
    pub order_by: SmallVec<[(NodeId, Order); 4]>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub lock: Option<LockHint>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InsertNode {
    pub target: StrId,
    pub columns: SmallVec<[StrId; 8]>,
    pub values: SmallVec<[NodeId; 8]>,
    pub returning: SmallVec<[NodeId; 4]>,
    pub conflict: Option<ConflictClause>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UpdateNode {
    pub target: StrId,
    pub columns: SmallVec<[StrId; 8]>,
    pub values: SmallVec<[NodeId; 8]>,
    pub filter: NodeId,
    pub returning: SmallVec<[NodeId; 4]>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeleteNode {
    pub target: StrId,
    pub filter: NodeId,
    pub returning: SmallVec<[NodeId; 4]>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UpsertNode {
    pub target: StrId,
    pub columns: SmallVec<[StrId; 8]>,
    pub values: SmallVec<[NodeId; 8]>,
    pub returning: SmallVec<[NodeId; 4]>,
    pub conflict: Option<ConflictClause>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum ExprNode {
    /// Static container address, up to (but not including) the leaf column.
    ///
    /// The address convention is backend-specific:
    ///
    /// | Backend    | Convention                    | Example              |
    /// |------------|-------------------------------|----------------------|
    /// | SQL        | `schema.table`                | `"public.users"`     |
    /// | Blob / S3  | `bucket/prefix`               | `"avatars/uploads"`  |
    /// | REST       | `/api/version/resource`       | `"/api/v2/users"`    |
    /// | KV         | namespace                     | `"sessions"`         |
    /// | Filesystem | directory path                | `"/var/data/"`       |
    ///
    /// The string is interned; use `ExprArena::alloc` with this variant to
    /// build a namespace expression.
    Namespace(StrId),
    /// A field reference: the leaf column/attribute name plus an optional
    /// chain of traversal steps (JSON key access or array indexing).
    ///
    /// The payload is stored in `ExprArena::fields`; this variant holds only
    /// the [`FieldId`] pool index, keeping `ExprNode` within its size budget.
    ///
    /// `ExprArena::fields`: crate::arena::ExprArena
    Field(FieldId),
    Param,
    /// A literal constant. The actual [`Literal`] is stored in `ExprArena::lits`;
    /// this variant holds only the pool index.
    ///
    /// [`Literal`]: crate::types::value::Literal
    Lit(LiteralId),
    /// An object literal. The field-pair list is stored in `ExprArena::obj_lits`;
    /// this variant holds only the pool index.
    ObjectLit(ObjLitId),
    ArrayLit(SmallVec<[NodeId; 4]>),
    BinOp {
        op: BinOp,
        lhs: NodeId,
        rhs: NodeId,
    },
    UnaryOp {
        op: UnaryOp,
        operand: NodeId,
    },
    /// A function call. The name and argument list are stored in `ExprArena::funcs`;
    /// this variant holds only the pool index.
    Func(FuncId),
    Agg {
        func: StrId,
        expr: NodeId,
        distinct: bool,
    },
    /// A window function. The payload is stored in `ExprArena::windows`;
    /// this variant holds only the pool index.
    Window(WindowId),
    Cast {
        expr: NodeId,
        to: StrId,
    },
    /// A `CASE WHEN … THEN … ELSE … END` expression. The payload is stored in
    /// `ExprArena::cases`; this variant holds only the pool index.
    Case(CaseId),
    Alias {
        expr: NodeId,
        name: StrId,
    },
    /// An `expr IN (list)` expression. The payload is stored in
    /// `ExprArena::in_lists`; this variant holds only the pool index.
    InList(InListId),
    InSub {
        expr: NodeId,
        sub: NodeId,
    },
    Exists {
        sub: NodeId,
    },
    IsNull {
        expr: NodeId,
    },
    Between {
        expr: NodeId,
        lo: NodeId,
        hi: NodeId,
    },
    /// A SELECT sub-query. The payload is stored in `ExprArena::queries`;
    /// this variant holds only the pool index.
    Query(QueryId),
    /// An INSERT statement. The payload is stored in `ExprArena::inserts`;
    /// this variant holds only the pool index.
    Insert(InsertId),
    /// An UPDATE statement. The payload is stored in `ExprArena::updates`;
    /// this variant holds only the pool index.
    Update(UpdateId),
    /// A DELETE statement. The payload is stored in `ExprArena::deletes`;
    /// this variant holds only the pool index.
    Delete(DeleteId),
    /// An UPSERT statement. The payload is stored in `ExprArena::upserts`;
    /// this variant holds only the pool index.
    Upsert(UpsertId),
}

#[cfg(test)]
mod size_tests {
    use core::mem::size_of;

    use super::ExprNode;
    use crate::types::value::{Literal, Value};

    #[test]
    fn expr_node_fits_32_bytes() {
        let sz = size_of::<ExprNode>();
        assert!(
            sz <= 32,
            "ExprNode is {sz} bytes on this target — must be ≤ 32; \
             pool a large variant via ExprArena",
        );
    }

    #[test]
    fn value_fits_24_bytes() {
        let sz = size_of::<Value>();
        assert!(sz <= 24, "Value is {sz} bytes — must be ≤ 24",);
    }

    #[test]
    fn literal_static_fits_32_bytes() {
        let sz = size_of::<Literal<'static>>();
        assert!(sz <= 32, "Literal<'static> is {sz} bytes — must be ≤ 32",);
    }
}

#[cfg(test)]
mod field_tests {
    use crate::arena::{ExprArena, FieldNode, FieldStep};
    use crate::session::BuildSession;
    use smallvec::smallvec;

    use super::ExprNode;

    // ── Arena-level field construction ───────────────────────────────────────

    #[test]
    fn bare_field_roundtrips() {
        let mut arena = ExprArena::new();
        let mut interner = crate::interner::Interner::new();

        let col_id = interner.intern("id");
        let fid = arena.alloc_field(FieldNode {
            namespace: None,
            name: col_id,
            steps: smallvec![],
        });
        let nid = arena.alloc(ExprNode::Field(fid));

        let ExprNode::Field(got_fid) = arena.get(nid) else {
            panic!("expected Field")
        };
        let node = arena.get_field(*got_fid);
        assert!(node.namespace.is_none());
        assert_eq!(node.name, col_id);
        assert!(node.steps.is_empty());
    }

    #[test]
    fn qualified_field_roundtrips() {
        let mut arena = ExprArena::new();
        let mut interner = crate::interner::Interner::new();

        let ns_id = interner.intern("public.users");
        let col_id = interner.intern("profile_json");
        let key_id = interner.intern("name");

        let fid = arena.alloc_field(FieldNode {
            namespace: Some(ns_id),
            name: col_id,
            steps: smallvec![FieldStep::Key(key_id)],
        });
        let nid = arena.alloc(ExprNode::Field(fid));

        let ExprNode::Field(got_fid) = arena.get(nid) else {
            panic!("expected Field")
        };
        let node = arena.get_field(*got_fid);
        assert_eq!(node.namespace, Some(ns_id));
        assert_eq!(node.name, col_id);
        assert_eq!(node.steps.len(), 1);
        assert_eq!(node.steps[0], FieldStep::Key(key_id));
    }

    #[test]
    fn deep_traversal_key_then_index() {
        // Represents: data->'meta'->>0  (SQL) / data.meta[0] (REST/doc)
        let mut arena = ExprArena::new();
        let mut interner = crate::interner::Interner::new();

        let col_id = interner.intern("data");
        let meta_id = interner.intern("meta");

        let fid = arena.alloc_field(FieldNode {
            namespace: None,
            name: col_id,
            steps: smallvec![FieldStep::Key(meta_id), FieldStep::Index(0)],
        });
        let nid = arena.alloc(ExprNode::Field(fid));

        let ExprNode::Field(got_fid) = arena.get(nid) else {
            panic!("expected Field")
        };
        let node = arena.get_field(*got_fid);
        assert_eq!(node.steps.len(), 2);
        assert_eq!(node.steps[0], FieldStep::Key(meta_id));
        assert_eq!(node.steps[1], FieldStep::Index(0));
    }

    #[test]
    fn namespace_node_roundtrips() {
        let mut arena = ExprArena::new();
        let mut interner = crate::interner::Interner::new();

        let path_id = interner.intern("public.users");
        let nid = arena.alloc(ExprNode::Namespace(path_id));

        let ExprNode::Namespace(got_id) = arena.get(nid) else {
            panic!("expected Namespace")
        };
        assert_eq!(*got_id, path_id);
    }

    // ── BuildSession helpers ─────────────────────────────────────────────────

    #[test]
    fn session_field_helper() {
        let mut sess = BuildSession::new();
        let nid = sess.field("email");

        let ExprNode::Field(fid) = sess.arena.get(nid) else {
            panic!("expected Field")
        };
        let node = sess.arena.get_field(*fid);
        assert!(node.namespace.is_none());
        assert_eq!(sess.interner.get(node.name), "email");
        assert!(node.steps.is_empty());
    }

    #[test]
    fn session_qualified_field_with_json_step() {
        // Represents: users.profile_json->>'name'
        let mut sess = BuildSession::new();
        let key_id = sess.intern("name");
        let nid = sess.qualified_field("users", "profile_json", smallvec![FieldStep::Key(key_id)]);

        let ExprNode::Field(fid) = sess.arena.get(nid) else {
            panic!("expected Field")
        };
        let node = sess.arena.get_field(*fid);
        assert_eq!(sess.interner.get(node.namespace.unwrap()), "users");
        assert_eq!(sess.interner.get(node.name), "profile_json");
        assert_eq!(node.steps.len(), 1);
        assert_eq!(node.steps[0], FieldStep::Key(key_id));
    }

    #[test]
    fn session_namespace_helper() {
        let mut sess = BuildSession::new();
        let nid = sess.namespace("public.users");

        let ExprNode::Namespace(id) = sess.arena.get(nid) else {
            panic!("expected Namespace")
        };
        assert_eq!(sess.interner.get(*id), "public.users");
    }

    #[test]
    fn session_deep_rest_path() {
        // Represents: response.data.items[0].name
        // namespace="response" (or no namespace if root), column="data",
        // steps=[Key("items"), Index(0), Key("name")]
        let mut sess = BuildSession::new();
        let items_id = sess.intern("items");
        let name_id = sess.intern("name");
        let nid = sess.qualified_field(
            "response",
            "data",
            smallvec![
                FieldStep::Key(items_id),
                FieldStep::Index(0),
                FieldStep::Key(name_id)
            ],
        );

        let ExprNode::Field(fid) = sess.arena.get(nid) else {
            panic!("expected Field")
        };
        let node = sess.arena.get_field(*fid);
        assert_eq!(sess.interner.get(node.namespace.unwrap()), "response");
        assert_eq!(sess.interner.get(node.name), "data");
        assert_eq!(node.steps.len(), 3);
        assert!(matches!(node.steps[0], FieldStep::Key(_)));
        assert_eq!(node.steps[1], FieldStep::Index(0));
        assert!(matches!(node.steps[2], FieldStep::Key(_)));
    }
}
