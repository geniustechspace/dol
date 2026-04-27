//! [`OpKind`] and [`Category`] — coarse and concrete classification of an
//! [`Operation`](super::Operation).
//!
//! The two enums let capability checks (`dol-ir::capabilities`) and
//! diagnostics (`dol-check`) reason about operations without matching on
//! every variant of the outer enum.

/// Coarse classification of an [`Operation`](super::Operation) by IR concern.
///
/// Used by capability tags and diagnostics that only need to distinguish
/// "this is a write" from "this is a read" or "this is access-control".
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

/// Concrete kind of an [`Operation`](super::Operation), used by capability
/// checks and diagnostics.
///
/// One variant per outer enum variant. Cheap to copy; cheap to match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OpKind {
    /// `Operation::Schema` — entity-level structural change.
    Schema,
    /// `Operation::Field` — field-level structural change.
    Field,
    /// `Operation::Index` — secondary index lifecycle.
    Index,
    /// `Operation::Lookup` — fast-membership index lifecycle.
    Lookup,
    /// `Operation::Policy` — row/object policy lifecycle.
    Policy,
    /// `Operation::Mask` — column / field masking rule lifecycle.
    Mask,
    /// `Operation::Quota` — usage quota rule lifecycle.
    Quota,
    /// `Operation::Audit` — auditing / observability rule lifecycle.
    Audit,
    /// `Operation::Insert`.
    Insert,
    /// `Operation::Update`.
    Update,
    /// `Operation::Replace`.
    Replace,
    /// `Operation::Delete`.
    Delete,
    /// `Operation::Upsert`.
    Upsert,
    /// `Operation::Append`.
    Append,
    /// `Operation::Query`.
    Query,
    /// `Operation::Probe`.
    Probe,
    /// `Operation::Describe`.
    Describe,
    /// `Operation::Grant`.
    Grant,
    /// `Operation::Revoke`.
    Revoke,
    /// `Operation::Tx`.
    Tx,
    /// `Operation::Extension`.
    Extension,
    /// `Operation::Raw` (feature-gated).
    #[cfg(feature = "raw")]
    Raw,
}

impl OpKind {
    /// Coarse [`Category`] for this kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use dol_ir::operation::{Category, OpKind};
    ///
    /// assert_eq!(OpKind::Insert.category(), Category::DML);
    /// assert_eq!(OpKind::Query.category(), Category::DQL);
    /// assert_eq!(OpKind::Grant.category(), Category::ACL);
    /// assert_eq!(OpKind::Schema.category(), Category::DDL);
    /// ```
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
            OpKind::Extension => Category::Other,
            #[cfg(feature = "raw")]
            OpKind::Raw => Category::Other,
        }
    }
}
