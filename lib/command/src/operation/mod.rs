//! Operation kinds, categories, and the outer [`Operation`] enum.
//!
//! `Operation` is the IR's universal dispatch enum. Variants follow a
//! noun/verb hybrid rule:
//!
//! - **Structural / governance** variants are nouns: `Schema`, `Field`,
//!   `Index`, `Lookup`, `Policy`, `Mask`, `Quota`, `Audit`. Their payloads
//!   carry a [`StructuralVerb`].
//! - **Data / query / authorization** variants are verbs: `Insert`, `Update`,
//!   `Replace`, `Delete`, `Upsert`, `Append`, `Query`, `Probe`, `Describe`,
//!   `Grant`, `Revoke`.
//! - **Meta** variants: `Tx`, `Extension`, `Raw` (feature-gated).
//!
//! Heavy payloads are held behind `Box` so `size_of::<Operation>()` stays
//! within the 64-byte budget enforced by `xtask size` and a const assertion
//! at the bottom of this file.

pub mod acl;
pub mod ddl;
pub mod dml;
pub mod dql;
pub mod kind;
pub mod meta;
pub mod shared;
pub mod tx;

use alloc::boxed::Box;

pub use acl::{
    AuditEvent, AuditOp, AuditSink, Grant, MaskOp, PolicyOp, PolicyScope, QuotaKind, QuotaOp,
    Revoke,
};
pub use ddl::{
    FieldDef, FieldOp, IndexDirection, IndexKey, IndexMethod, IndexOp, LookupMethod, LookupOp,
    SchemaBody, SchemaOp, TypeBody,
};
pub use dml::{Append, Delete, Insert, InsertSource, Replace, ReplaceBody, Update, Upsert};
pub use dql::{Describe, DescribeFacet, Probe, Query};
pub use kind::{Category, OpKind};
#[cfg(feature = "raw")]
pub use meta::RawOp;
pub use meta::{ExtensionId, ExtensionPayload, OperationExtension};
pub use shared::StructuralVerb;
pub use tx::{IsolationLevel, TxBegin, TxOp, TxOptions};

/// Top-level IR operation.
///
/// See the module docs for the noun-vs-verb variant rule and `docs/IR.md` for
/// the design overview.
///
/// # Examples
///
/// ```
/// use dol_command::operation::{Insert, InsertSource, Operation};
/// use dol_command::target::{Locator, Symbol, Target, TargetKind};
/// use dol_command::operation::OpKind;
///
/// // Create an Insert operation for a SQL table
/// let op: Operation = Insert {
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(0))),
///     source: InsertSource::Bindings,
///     returning: None,
/// }
/// .into();
///
/// assert_eq!(op.kind(), OpKind::Insert);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Operation {
    // ── structural / management — noun variants ───────────────────────────
    /// Entity-level structural change (create/drop/alter/rename a relation,
    /// document collection, blob bucket, etc.).
    Schema(Box<SchemaOp>),
    /// Field-level structural change (add/drop/alter/rename a column on a
    /// relation or property on a document collection).
    Field(Box<FieldOp>),
    /// Secondary index lifecycle (create/drop/alter an index on a target).
    Index(Box<IndexOp>),
    /// Fast-membership lookup lifecycle (create/drop a lookup on a target).
    Lookup(Box<LookupOp>),

    // ── ACL / governance — noun variants ──────────────────────────────────
    /// Row-level security / authorisation policy lifecycle.
    Policy(Box<PolicyOp>),
    /// Column / field masking rule lifecycle.
    Mask(Box<MaskOp>),
    /// Usage quota / rate-limit rule lifecycle.
    Quota(Box<QuotaOp>),
    /// Auditing / observability rule lifecycle.
    Audit(Box<AuditOp>),

    // ── DML — verb variants ───────────────────────────────────────────────
    /// Insert new tuples / documents / objects.
    Insert(Box<Insert>),
    /// Partial mutation of existing rows / documents.
    Update(Box<Update>),
    /// Full overwrite of rows / objects (HTTP PUT semantics).
    Replace(Box<Replace>),
    /// Remove rows / documents / objects.
    Delete(Box<Delete>),
    /// Insert-or-update with conflict resolution.
    Upsert(Box<Upsert>),
    /// Append-only write (stream topics, immutable logs).
    Append(Box<Append>),

    // ── DQL — verb variants ───────────────────────────────────────────────
    /// Read tuples / documents / objects / files.
    Query(Box<Query>),
    /// Existence / metadata check (HTTP HEAD, S3 HeadObject).
    Probe(Box<Probe>),
    /// Schema introspection (SHOW TABLES, information_schema).
    Describe(Box<Describe>),

    // ── ACL actions — verb variants ───────────────────────────────────────
    /// Bestow privileges on a target to roles.
    Grant(Box<Grant>),
    /// Withdraw privileges on a target from roles.
    Revoke(Box<Revoke>),

    // ── meta ──────────────────────────────────────────────────────────────
    /// Transaction control (begin/commit/rollback/savepoint).
    Tx(Box<TxOp>),
    /// Open extension payload for higher-level crates.
    Extension(Box<OperationExtension>),
    /// Feature-gated escape hatch for pre-built dialect-specific statements.
    #[cfg(feature = "raw")]
    Raw(Box<RawOp>),
}

impl Operation {
    /// Concrete kind of this operation.
    pub fn kind(&self) -> OpKind {
        match self {
            Operation::Schema(_) => OpKind::Schema,
            Operation::Field(_) => OpKind::Field,
            Operation::Index(_) => OpKind::Index,
            Operation::Lookup(_) => OpKind::Lookup,
            Operation::Policy(_) => OpKind::Policy,
            Operation::Mask(_) => OpKind::Mask,
            Operation::Quota(_) => OpKind::Quota,
            Operation::Audit(_) => OpKind::Audit,
            Operation::Insert(_) => OpKind::Insert,
            Operation::Update(_) => OpKind::Update,
            Operation::Replace(_) => OpKind::Replace,
            Operation::Delete(_) => OpKind::Delete,
            Operation::Upsert(_) => OpKind::Upsert,
            Operation::Append(_) => OpKind::Append,
            Operation::Query(_) => OpKind::Query,
            Operation::Probe(_) => OpKind::Probe,
            Operation::Describe(_) => OpKind::Describe,
            Operation::Grant(_) => OpKind::Grant,
            Operation::Revoke(_) => OpKind::Revoke,
            Operation::Tx(_) => OpKind::Tx,
            Operation::Extension(_) => OpKind::Extension,
            #[cfg(feature = "raw")]
            Operation::Raw(_) => OpKind::Raw,
        }
    }

    /// Coarse category.
    #[inline]
    pub fn category(&self) -> Category {
        self.kind().category()
    }

    /// The primary [`Target`](crate::target::Target) of this operation, if
    /// any. Operations without a single primary target (`Tx`, `Extension`,
    /// `Raw`) return `None`.
    ///
    /// **Non-recursive:** for `Operation::Tx(TxOp::Atomic { ops, .. })`
    /// this returns `None`. Use [`Operation::all_targets`] when you need to
    /// walk into atomic transaction bodies (capability checks, ACL gates,
    /// catalog resolution).
    pub fn primary_target(&self) -> Option<&crate::target::Target> {
        match self {
            Operation::Schema(p) => Some(&p.target),
            Operation::Field(p) => Some(&p.target),
            Operation::Index(p) => Some(&p.target),
            Operation::Lookup(p) => Some(&p.target),
            Operation::Policy(p) => Some(&p.target),
            Operation::Mask(p) => Some(&p.target),
            Operation::Quota(p) => Some(&p.target),
            Operation::Audit(p) => Some(&p.target),
            Operation::Insert(p) => Some(&p.target),
            Operation::Update(p) => Some(&p.target),
            Operation::Replace(p) => Some(&p.target),
            Operation::Delete(p) => Some(&p.target),
            Operation::Upsert(p) => Some(&p.target),
            Operation::Append(p) => Some(&p.target),
            Operation::Query(p) => Some(&p.target),
            Operation::Probe(p) => Some(&p.target),
            Operation::Describe(p) => Some(&p.target),
            Operation::Grant(p) => Some(&p.target),
            Operation::Revoke(p) => Some(&p.target),
            Operation::Tx(_) | Operation::Extension(_) => None,
            #[cfg(feature = "raw")]
            Operation::Raw(_) => None,
        }
    }

    /// Every [`Target`](crate::target::Target) reachable from this operation,
    /// including those nested inside `TxOp::Atomic { ops, .. }`. The order
    /// is depth-first, parent before children, mirroring program order.
    ///
    /// Use this from capability checks, ACL gates, and catalog resolution
    /// — anywhere the bare [`primary_target`](Self::primary_target) would
    /// silently miss the targets nested in an atomic transaction body.
    pub fn all_targets(&self) -> alloc::vec::Vec<&crate::target::Target> {
        let mut out: alloc::vec::Vec<&crate::target::Target> = alloc::vec::Vec::new();
        self.collect_targets(&mut out);
        out
    }

    fn collect_targets<'a>(&'a self, out: &mut alloc::vec::Vec<&'a crate::target::Target>) {
        if let Some(t) = self.primary_target() {
            out.push(t);
            return;
        }
        if let Operation::Tx(tx) = self {
            if let TxOp::Atomic { ops, .. } = tx.as_ref() {
                for op in ops {
                    op.collect_targets(out);
                }
            }
        }
    }
}

// ── Ergonomic constructors via `From<Payload> for Operation` ──────────────

macro_rules! impl_op_from {
    ($( $variant:ident($payload:ty) ; )+) => {
        $(
            impl From<$payload> for Operation {
                #[inline]
                fn from(value: $payload) -> Self {
                    Operation::$variant(Box::new(value))
                }
            }
        )+
    };
}

impl_op_from! {
    Schema(SchemaOp);
    Field(FieldOp);
    Index(IndexOp);
    Lookup(LookupOp);
    Policy(PolicyOp);
    Mask(MaskOp);
    Quota(QuotaOp);
    Audit(AuditOp);
    Insert(Insert);
    Update(Update);
    Replace(Replace);
    Delete(Delete);
    Upsert(Upsert);
    Append(Append);
    Query(Query);
    Probe(Probe);
    Describe(Describe);
    Grant(Grant);
    Revoke(Revoke);
    Tx(TxOp);
    Extension(OperationExtension);
}

#[cfg(feature = "raw")]
impl From<RawOp> for Operation {
    #[inline]
    fn from(value: RawOp) -> Self {
        Operation::Raw(Box::new(value))
    }
}

// ── Static invariants ─────────────────────────────────────────────────────

const _: () = {
    assert!(
        core::mem::size_of::<Operation>() <= 64,
        "Operation must fit within the 64-byte size budget"
    );
};

const _: fn() = || {
    fn assert_send_sync_static<T: Send + Sync + 'static>() {}
    assert_send_sync_static::<Operation>();
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::{Locator, Symbol, Target, TargetKind};

    #[test]
    fn size_is_within_budget() {
        assert!(core::mem::size_of::<Operation>() <= 64);
    }

    #[test]
    fn kind_and_category() {
        let op: Operation = Insert {
            target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(0))),
            source: InsertSource::Bindings,
            returning: None,
        }
        .into();
        assert_eq!(op.kind(), OpKind::Insert);
        assert_eq!(op.category(), Category::DML);
    }

    #[test]
    fn primary_target_for_data_ops() {
        let op: Operation = Insert {
            target: Target::new(TargetKind::Blob, Locator::new(Symbol::from_hash(7))),
            source: InsertSource::Bindings,
            returning: None,
        }
        .into();
        assert!(matches!(
            op.primary_target().map(|t| &t.kind),
            Some(TargetKind::Blob)
        ));
    }

    #[test]
    fn all_targets_recurses_into_tx_atomic() {
        use crate::operation::tx::{TxOp, TxOptions};

        let inner_a: Operation = Insert {
            target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(1))),
            source: InsertSource::Bindings,
            returning: None,
        }
        .into();
        let inner_b: Operation = Insert {
            target: Target::new(TargetKind::Blob, Locator::new(Symbol::from_hash(2))),
            source: InsertSource::Bindings,
            returning: None,
        }
        .into();
        let atomic: Operation = TxOp::Atomic {
            ops: alloc::vec![inner_a, inner_b],
            opts: TxOptions::default(),
        }
        .into();

        // primary_target does NOT recurse.
        assert!(atomic.primary_target().is_none());
        // all_targets DOES.
        let kinds: alloc::vec::Vec<&TargetKind> =
            atomic.all_targets().iter().map(|t| &t.kind).collect();
        assert_eq!(kinds.len(), 2);
        assert!(matches!(kinds[0], TargetKind::Relation));
        assert!(matches!(kinds[1], TargetKind::Blob));
    }
}
