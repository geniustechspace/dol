//! Zero-sized tag markers for [`Lid`](super::Lid) / [`Cid`](super::Cid) /
//! [`Gid`](super::Gid).
//!
//! Tags are uninhabited `enum`s with no variants — they exist
//! purely at the type level so different pools / arenas produce
//! distinct handle types. Per `dol-rewrite-plan-v2.md` §7.2.

/// Tag for interned-string handles.
///
/// Re-exported from [`dol_core::strings::StrTag`] so that
/// [`super::StrId`] is structurally identical to
/// [`dol_core::strings::StrId`]. This lets `dol-core`'s
/// `impl PathSegment for StrId` apply directly to handles issued by
/// `dol-cas`'s [`crate::string_pool::StringPool`] — the closing of
/// the two-mode path bridge per `dol-rewrite-plan-v2.md` §7.5.
pub use dol_core::strings::StrTag;

/// Tag for expression-node handles.
pub enum NodeTag {}

/// Tag for field-chain handles.
pub enum FieldTag {}

/// Tag for entity-table handles.
pub enum EntityTag {}

/// Tag for interned-literal handles.
pub enum LiteralTag {}

/// Tag for function / opcode handles.
pub enum FuncTag {}

/// Tag for schema handles. No `Lid` alias — schemas are addressed by
/// content (see [`super::SchemaCid`]).
pub enum SchemaTag {}

/// Tag for program handles. No `Lid` alias — programs are addressed by
/// content (see [`super::ProgramCid`] / [`super::ProgramGid`]).
pub enum ProgramTag {}
