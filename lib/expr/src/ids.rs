//! Typed identifier newtypes used throughout the expression IR.
//!
//! Each id is `dol_core::id::Id<…Tag>`, a 32-bit handle backed by
//! [`core::num::NonZeroU32`] so `Option<…Id>` is exactly four bytes
//! (niche-optimised). Tagging the ids by purpose prevents accidental
//! cross-pool aliasing (`NodeId` vs `FieldId`) at the type level while
//! preserving the cache-friendly representation.
//!
//! There is **no** `NULL_NODE` sentinel; "missing" ids are encoded as
//! `Option<…Id>`, which costs the same four bytes thanks to the niche.

pub use dol_core::id::Id;

/// Tag for `ExprArena::nodes` ids. Phantom marker; never instantiated.
pub enum NodeTag {}
/// Index into `ExprArena::nodes`.
pub type NodeId = Id<NodeTag>;

/// Tag for [`crate::Interner`] string ids. Phantom marker.
pub enum StrTag {}
/// Content-addressed id for an interned string (leading 32 bits of
/// BLAKE3, stored one-based to fit the `NonZeroU32` niche).
pub type StrId = Id<StrTag>;

/// Tag for `TypeArena::types` ids. Phantom marker.
pub enum TypeTag {}
/// Index into `TypeArena::types`.
pub type TypeId = Id<TypeTag>;

/// Tag for `SpanTable::spans` ids. Phantom marker.
pub enum SpanTag {}
/// Index into `SpanTable::spans`.
pub type SpanId = Id<SpanTag>;

/// Tag for `ExprArena::lits` ids. Phantom marker.
pub enum LiteralTag {}
/// Index into `ExprArena::lits` — identifies a pooled
/// [`crate::types::Literal<'static>`].
pub type LiteralId = Id<LiteralTag>;

/// Tag for `ExprArena::funcs` ids. Phantom marker.
pub enum FuncTag {}
/// Index into `ExprArena::funcs` — identifies a pooled
/// [`crate::FuncNode`].
pub type FuncId = Id<FuncTag>;

/// Tag for `ExprArena::windows` ids. Phantom marker.
pub enum WindowTag {}
/// Index into `ExprArena::windows` — identifies a pooled
/// [`crate::WindowNode`].
pub type WindowId = Id<WindowTag>;

/// Tag for `ExprArena::cases` ids. Phantom marker.
pub enum CaseTag {}
/// Index into `ExprArena::cases` — identifies a pooled
/// [`crate::CaseNode`].
pub type CaseId = Id<CaseTag>;

/// Tag for `ExprArena::composites` ids. Phantom marker.
pub enum CompositeTag {}
/// Index into `ExprArena::composites` — identifies a pooled
/// [`crate::CompositeNode`] (the unified array / object / tuple
/// container introduced when [`crate::expr::ExprOp::ObjectLit`] and
/// [`crate::expr::ExprOp::ArrayLit`] were collapsed onto a single
/// [`crate::expr::ExprOp::Composite`] opcode).
///
/// The same pool also backs the row form of `IN (a, b, c)` via
/// [`crate::expr::ExprOp::In`]: the right-hand collection is a
/// [`crate::expr::ExprOp::Composite`] node whose `kind` is
/// `Array` (one column) or `Tuple` (multi-column row).
pub type CompositeId = Id<CompositeTag>;

/// Tag for `ExprArena::queries` ids. Phantom marker.
pub enum QueryTag {}
/// Index into `ExprArena::queries` — identifies a pooled
/// [`crate::QueryNode`].
pub type QueryId = Id<QueryTag>;

/// Tag for `ExprArena::inserts` ids. Phantom marker.
pub enum InsertTag {}
/// Index into `ExprArena::inserts` — identifies a pooled
/// [`crate::InsertNode`].
pub type InsertId = Id<InsertTag>;

/// Tag for `ExprArena::updates` ids. Phantom marker.
pub enum UpdateTag {}
/// Index into `ExprArena::updates` — identifies a pooled
/// [`crate::UpdateNode`].
pub type UpdateId = Id<UpdateTag>;

/// Tag for `ExprArena::deletes` ids. Phantom marker.
pub enum DeleteTag {}
/// Index into `ExprArena::deletes` — identifies a pooled
/// [`crate::DeleteNode`].
pub type DeleteId = Id<DeleteTag>;

/// Tag for `ExprArena::upserts` ids. Phantom marker.
pub enum UpsertTag {}
/// Index into `ExprArena::upserts` — identifies a pooled
/// [`crate::UpsertNode`].
pub type UpsertId = Id<UpsertTag>;

/// Tag for `ExprArena::fields` ids. Phantom marker.
pub enum FieldTag {}
/// Index into `ExprArena::fields` — identifies a pooled
/// [`crate::FieldNode`].
pub type FieldId = Id<FieldTag>;
