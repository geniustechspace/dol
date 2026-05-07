//! `Audit` — auditing / observability rule lifecycle.

use crate::operation::shared::StructuralVerb;
use crate::target::{Symbol, Target};

/// What event class to audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum AuditEvent {
    /// Audit read operations (SELECT, Query).
    Read,
    /// Audit write operations (INSERT, UPDATE, DELETE).
    Write,
    /// Audit schema changes (CREATE, ALTER, DROP).
    SchemaChange,
    /// Audit access control changes (GRANT, REVOKE).
    AccessControl,
    /// Audit all event classes.
    All,
}

/// Where audit records are written.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum AuditSink {
    /// Backend's default audit log.
    Default,
    /// A named sink registered by the backend (e.g. `kafka://audit-topic`).
    Named(Symbol),
}

/// `Audit` operation.
///
/// Defines an auditing rule that records events on a target to a sink.
///
/// # Examples
///
/// ```
/// use dol_ir::operation::{AuditEvent, AuditOp, AuditSink, StructuralVerb};
/// use dol_ir::target::{Locator, Symbol, Target, TargetKind};
/// use dol_ir::operation::Operation;
///
/// // Audit all writes on the "users" table
/// let op: Operation = AuditOp {
///     verb: StructuralVerb::Create,
///     target: Target::new(TargetKind::Relation, Locator::new(Symbol::from_hash(0))),
///     name: Symbol::from_hash(1),
///     event: AuditEvent::Write,
///     sink: AuditSink::Default,
/// }
/// .into();
///
/// assert_eq!(op.kind(), dol_ir::operation::OpKind::Audit);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct AuditOp {
    /// `Create` / `Drop` / `Alter`.
    pub verb: StructuralVerb,
    /// Target to audit.
    pub target: Target,
    /// Interned audit rule name.
    pub name: Symbol,
    /// Event class to capture.
    pub event: AuditEvent,
    /// Destination for audit records.
    pub sink: AuditSink,
}
