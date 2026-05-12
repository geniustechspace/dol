//! # `dol-ir` — intermediate representation
//!
//! Per `dol-rewrite-plan-v2.md` §8. M3 is the largest milestone in
//! the rewrite and is split across PRs **M3a–M3e**.
//!
//! ## What ships in M3a + M3b
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
//!   addressed by [`NodeId`](dol_cas::handle::NodeId). Two construction
//!   modes: raw [`push`](expr::arena::ExprArena::push) (no dedup) and
//!   [`intern_node`](expr::arena::ExprArena::intern_node) (structural
//!   dedup keyed by `fast64` over the 16-byte node).
//! - [`expr::walk::content_hash`] — bottom-up walker that derives the
//!   BLAKE3-128 content address of any subtree, memoised in
//!   [`dol_cas::content_index::ContentIndex`]. Threads `&mut Budget`
//!   per descent; cache hits are free.
//!
//! ## What lands in M3c–M3e
//!
//! - **M3c**: tree DSL `Expr<'a>` (§8.2), lowering `Expr<'a> →
//!   ExprArena` with `lower_path` (§8.4 main body), and the
//!   `Context<'a>` / `OrderByExpr` / `Frame` family (§8.3).
//! - **M3d**: schema types — `Entity`, `Field`, `SchemaCatalog` (§8.6).
//! - **M3e**: `Operation` / `Program` (§8.7), `Backend` trait + reference
//!   no-op backend (§8.8), and optional `stream` / `pipeline` features
//!   (§8.9–§8.10).
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
