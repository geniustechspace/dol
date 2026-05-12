//! Expression IR — the flat 16-byte node arena that backs every
//! `dol-ir` expression representation.
//!
//! Per `dol-rewrite-plan-v2.md` §8.1.
//!
//! - [`node::ExprNode`] — the 16-byte POD record itself.
//! - [`ops`] — opcode tables ([`OpFamily`](ops::OpFamily),
//!   [`BinOp`](ops::BinOp), [`UnaryOp`](ops::UnaryOp)) with the
//!   append-only numeric-stability contract.
//! - [`flags::NodeFlags`] — per-node bitset (`nullable` / `distinct`
//!   / `negated` / `aggregate`).
//! - [`arena::ExprArena`] — flat `DynPool<ExprNode>` store keyed by
//!   [`NodeId`](dol_cas::handle::NodeId). Std-only; M3a ships push/get
//!   without dedup (lands in M3b alongside lowering).

pub mod arena;
pub mod flags;
pub mod node;
pub mod ops;
pub mod walk;
