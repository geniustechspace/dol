//! Access-control (ACL) and governance operation payloads.
//!
//! Lifecycle objects (nouns: `Policy`, `Mask`, `Quota`, `Audit`) carry a
//! [`StructuralVerb`](super::shared::StructuralVerb). Lifecycle actions
//! (verbs: `Grant`, `Revoke`) carry the [`Privilege`](crate::privilege::Privilege) set
//! they bestow / withdraw.

pub mod audit;
pub mod grant;
pub mod mask;
pub mod policy;
pub mod quota;
pub mod revoke;

pub use audit::{AuditEvent, AuditOp, AuditSink};
pub use grant::Grant;
pub use mask::MaskOp;
pub use policy::{PolicyOp, PolicyScope};
pub use quota::{QuotaKind, QuotaOp};
pub use revoke::Revoke;
