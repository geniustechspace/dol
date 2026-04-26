//! Operation kinds, categories, and the outer [`Operation`] enum.
//!
//! `Operation` is the v2 IR's universal dispatch enum. Variants follow the
//! noun/verb hybrid rule from `docs/rfcs/0001-ir-v2.md`:
//!
//! - **Structural / governance** variants are nouns: `Schema`, `Field`,
//!   `Index`, `Lookup`, `Policy`, `Mask`, `Quota`, `Audit`. Their payloads
//!   carry a [`StructuralVerb`](schema::StructuralVerb).
//! - **Data / query / authorization** variants are verbs: `Insert`, `Update`,
//!   `Replace`, `Delete`, `Upsert`, `Append`, `Query`, `Probe`, `Describe`,
//!   `Grant`, `Revoke`.
//! - **Meta** variants: `Tx`, `Extension`, `Raw` (feature-gated).
//!
//! Heavy payloads are held behind `Box` so `size_of::<Operation>()` stays
//! within the 64-byte budget enforced by `xtask size` and a const assertion
//! at the bottom of this file.

pub mod append;
pub mod audit;
pub mod delete;
pub mod describe;
pub mod extension;
pub mod field;
pub mod grant;
pub mod index;
pub mod insert;
pub mod lookup;
pub mod mask;
pub mod policy;
pub mod probe;
pub mod query;
pub mod quota;
#[cfg(feature = "raw")]
pub mod raw;
pub mod replace;
pub mod revoke;
pub mod schema;
pub mod tx;
pub mod update;
pub mod upsert;

pub use append::Append;
pub use audit::{AuditEvent, AuditOp, AuditSink};
pub use delete::Delete;
pub use describe::{Describe, DescribeFacet};
pub use extension::{ExtensionId, OperationExtension};
pub use field::{FieldDefV2, FieldOp};
pub use grant::GrantV2;
pub use index::{IndexDirection, IndexKey, IndexMethod, IndexOp};
pub use insert::{Insert, InsertSource};
pub use lookup::{LookupMethod as LookupOpMethod, LookupOp};
pub use mask::MaskOp;
pub use policy::{PolicyOp, PolicyScope};
pub use probe::Probe;
pub use query::Query;
pub use quota::{QuotaKind, QuotaOp};
#[cfg(feature = "raw")]
pub use raw::RawOp;
pub use replace::{Replace, ReplaceBody};
pub use revoke::RevokeV2;
pub use schema::{SchemaBody, SchemaOp, StructuralVerb, TypeBody};
pub use tx::{IsolationLevel, TxBegin, TxOp, TxOptions};
pub use update::{Update, UpdateAssignment};
pub use upsert::{OnConflict, Upsert};

/// Top-level v2 IR operation.
///
/// See the module docs for the noun-vs-verb variant rule and `docs/IR.md` for
/// the design overview.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Operation {
    // ── structural / management — noun variants ───────────────────────────
    Schema(Box<SchemaOp>),
    Field(Box<FieldOp>),
    Index(Box<IndexOp>),
    Lookup(Box<LookupOp>),

    // ── ACL / governance — noun variants ──────────────────────────────────
    Policy(Box<PolicyOp>),
    Mask(Box<MaskOp>),
    Quota(Box<QuotaOp>),
    Audit(Box<AuditOp>),

    // ── DML — verb variants ───────────────────────────────────────────────
    Insert(Box<Insert>),
    Update(Box<Update>),
    Replace(Box<Replace>),
    Delete(Box<Delete>),
    Upsert(Box<Upsert>),
    Append(Box<Append>),

    // ── DQL — verb variants ───────────────────────────────────────────────
    Query(Box<Query>),
    Probe(Box<Probe>),
    Describe(Box<Describe>),

    // ── ACL actions — verb variants ───────────────────────────────────────
    Grant(Box<GrantV2>),
    Revoke(Box<RevokeV2>),

    // ── meta ──────────────────────────────────────────────────────────────
    Tx(Box<TxOp>),
    Extension(Box<OperationExtension>),
    #[cfg(feature = "raw")]
    Raw(Box<RawOp>),
}

/// Coarse classification of an [`Operation`] by IR concern.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::upper_case_acronyms)]
pub enum Category {
    /// Structural / management (`Schema`, `Field`, `Index`, `Lookup`).
    DDL,
    /// Data manipulation (`Insert`, `Update`, `Replace`, `Delete`, `Upsert`,
    /// `Append`).
    DML,
    /// Read / inspection (`Query`, `Probe`, `Describe`).
    DQL,
    /// Access control (`Grant`, `Revoke`, `Policy`, `Mask`, `Quota`, `Audit`).
    ACL,
    /// Transaction control (`Tx`).
    Tx,
    /// Out-of-band escape hatches (`Extension`, `Raw`).
    Other,
}

/// Concrete kind of an [`Operation`], used by capability checks and
/// diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OpKind {
    Schema,
    Field,
    Index,
    Lookup,
    Policy,
    Mask,
    Quota,
    Audit,
    Insert,
    Update,
    Replace,
    Delete,
    Upsert,
    Append,
    Query,
    Probe,
    Describe,
    Grant,
    Revoke,
    Tx,
    Extension,
    Raw,
}

impl OpKind {
    /// Coarse [`Category`] for this kind.
    pub const fn category(self) -> Category {
        match self {
            OpKind::Schema | OpKind::Field | OpKind::Index | OpKind::Lookup => Category::DDL,
            OpKind::Insert
            | OpKind::Update
            | OpKind::Replace
            | OpKind::Delete
            | OpKind::Upsert
            | OpKind::Append => Category::DML,
            OpKind::Query | OpKind::Probe | OpKind::Describe => Category::DQL,
            OpKind::Policy
            | OpKind::Mask
            | OpKind::Quota
            | OpKind::Audit
            | OpKind::Grant
            | OpKind::Revoke => Category::ACL,
            OpKind::Tx => Category::Tx,
            OpKind::Extension | OpKind::Raw => Category::Other,
        }
    }
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
    Grant(GrantV2);
    Revoke(RevokeV2);
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
            target: Target::new(TargetKind::Relation, Locator::new(Symbol::new(0))),
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
            target: Target::new(TargetKind::Blob, Locator::new(Symbol::new(7))),
            source: InsertSource::Bindings,
            returning: None,
        }
        .into();
        assert!(matches!(
            op.primary_target().map(|t| &t.kind),
            Some(TargetKind::Blob)
        ));
    }
}
