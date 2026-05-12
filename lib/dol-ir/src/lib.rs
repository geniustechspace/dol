//! # `dol-ir` — intermediate representation
//!
//! Per `dol-rewrite-plan-v2.md` §8. M3 is the largest milestone in
//! the rewrite and is split across PRs **M3a–M3e**.
//!
//! ## What ships in M3a (this revision)
//!
//! - [`expr::node::ExprNode`] — the 16-byte POD expression record.
//!   `#[repr(C)]`, `bytemuck::Pod`, const-asserted to be exactly
//!   16 bytes. This is the cornerstone of the v2 IR.
//! - [`expr::ops`] — opcode tables: [`expr::ops::OpFamily`] (top-level
//!   discriminator), [`expr::ops::BinOp`], [`expr::ops::UnaryOp`].
//!   All `#[non_exhaustive]` with `try_from_*` decoders. Numeric
//!   stability is locked by the snapshot tests in `expr::ops::tests`.
//! - [`expr::flags::NodeFlags`] — per-node bitset (`nullable` /
//!   `distinct` / `negated` / `aggregate`).
//! - [`expr::arena::ExprArena`] — flat `DynPool<ExprNode>` store
//!   addressed by [`NodeId`](dol_cas::handle::NodeId). Raw push/get
//!   surface only; dedup index lands in M3b.
//!
//! ## What lands in M3b–M3e
//!
//! - **M3b**: tree DSL `Expr<'a>` (§8.2), lowering `Expr<'a> →
//!   ExprArena` with `lower_path` (§8.4), arena dedup index, and the
//!   `ContentIndex` walker (§8.5).
//! - **M3c**: schema types — `Entity`, `Field`, `SchemaCatalog` (§8.6).
//! - **M3d**: `Operation` enum and `Program` (§8.7).
//! - **M3e**: `Backend` trait + reference no-op backend (§8.8) plus
//!   optional `stream` / `pipeline` features (§8.9–§8.10).
//!
//! ## Encapsulation rule
//!
//! Outside of [`expr::node`], **never** read or write `ExprNode`'s
//! `a` / `b` / `c` fields directly. Every consumer of an `ExprNode`
//! must go through one of the typed `ExprNode::as_*` accessors so
//! that the opcode↔operand contract is enforced by the type system.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub mod expr;
