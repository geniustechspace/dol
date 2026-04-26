//! `Audit` — auditing / observability rule lifecycle.

use crate::operation::shared::StructuralVerb;
use crate::target::{Symbol, Target};

/// What event class to audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AuditEvent {
    Read,
    Write,
    SchemaChange,
    AccessControl,
    All,
}

/// Where audit records are written.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AuditSink {
    /// Backend's default audit log.
    Default,
    /// A named sink registered by the backend.
    Named(Symbol),
}

/// `Audit` operation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AuditOp {
    pub verb: StructuralVerb,
    pub target: Target,
    pub name: Symbol,
    pub event: AuditEvent,
    pub sink: AuditSink,
}
