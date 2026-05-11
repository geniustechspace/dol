//! Zero-sized tag markers for [`Lid`](super::Lid) / [`Cid`](super::Cid) /
//! [`Gid`](super::Gid).
//!
//! Tags are uninhabited `enum`s with no variants — they exist
//! purely at the type level so different pools / arenas produce
//! distinct handle types. Per `dol-rewrite-plan-v2.md` §7.2.

/// Tag for interned-string handles.
pub enum StrTag {}

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
