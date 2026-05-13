//! # `dol-ir` — intermediate representation
//!
//! Per `dol-rewrite-plan-v2.md` §8. M3 is the largest milestone in
//! the rewrite and is split across PRs **M3a–M3e**.
//!
//! ## What ships in M3a + M3b + M3c (complete: α + β + γ + δ₁ + δ₂)
//!
//! `M3c` — the entire `expr::*` slice — is now feature-complete: the
//! recursive `lower(Expr<'a>) → ExprArena` pass and its `compute_hash`
//! integration are wired in alongside the supporting side pools.
//!
//! - [`expr::node::ExprNode`] — the 16-byte POD expression record.
//!   `#[repr(C)]`, `bytemuck::Pod`, const-asserted to be exactly
//!   16 bytes.
//! - [`expr::ops`] — opcode tables: [`expr::ops::OpFamily`] (now 20
//!   variants `Reserved`/`Bin`/`Unary`/`LitRef`/`FieldRef`/`FuncRef`/
//!   `Param`/`Wildcard`/`CountAll`/`PathRef`/`Seq`/`Map`/`Call`/
//!   `Cast`/`Match`/`If`/`InRange`/`MemberOf`/`Label`/`Scoped`,
//!   numerically locked by snapshot tests),
//!   [`expr::ops::BinOp`], [`expr::ops::UnaryOp`].
//! - [`expr::flags::NodeFlags`] — per-node bitset.
//! - [`expr::arena::ExprArena`] — flat `DynPool<ExprNode>` store
//!   addressed by [`NodeId`](dol_cas::handle::NodeId), bundling **six**
//!   side pools — [`literals`](expr::arena::ExprArena::literals) /
//!   [`funcs`](expr::arena::ExprArena::funcs) /
//!   [`operands`](expr::arena::ExprArena::operands) /
//!   [`paths`](expr::arena::ExprArena::paths) /
//!   [`types`](expr::arena::ExprArena::types) /
//!   [`contexts`](expr::arena::ExprArena::contexts) — so the
//!   recursive lowerer keeps the documented four-argument signature
//!   `lower(expr, arena, strings, budget)` (plan §8.4 line 1376).
//!   Two construction modes survive unchanged: raw
//!   [`push`](expr::arena::ExprArena::push) (no dedup) and
//!   [`intern_node`](expr::arena::ExprArena::intern_node) (structural
//!   dedup keyed by `fast64` over the 16-byte node).
//! - [`expr::walk::content_hash`] — bottom-up walker that derives the
//!   BLAKE3-128 content address of any subtree, memoised in
//!   [`dol_cas::content_index::ContentIndex`]. Now covers every
//!   `OpFamily` family: variadic spans (Seq/Map/Call/Match/MemberOf)
//!   recurse into operand-slab slots, `Scoped` folds in the lowered
//!   context (partition_by + order_by + frame), and `Cast` mixes in
//!   the type id. Cross-arena structural equivalence yields identical
//!   digests (locked by tests).
//! - [`expr::meta`] — tree-DSL leaf metadata: [`expr::meta::OpDef`]
//!   and [`expr::meta::FuncDef`].
//! - [`expr::frame`] — window / scope frame primitives.
//! - [`expr::order`] — ordering primitives plus
//!   [`expr::order::OrderByExpr`].
//! - [`expr::tree`] — the user-facing builder enum
//!   [`expr::tree::Expr`] with all 14 variants from plan §8.2.
//! - [`expr::context`] — [`expr::context::Context`] descriptor plus
//!   the fluent [`expr::context::ContextBuilder`] /
//!   [`expr::context::ConditionalBuilder`].
//! - [`expr::lower`] — the **single, authorised** lowering site.
//!   [`expr::lower::lower`] is the recursive `Expr<'a> → NodeId`
//!   pass; [`expr::lower::lower_path`] is the only `Path<Name> →
//!   Path<StrId>` conversion in the workspace (plan §8.4 line 1389).
//!   Charges one `Budget::depth()` per recursive descent and one
//!   `Budget::node()` per allocated arena node;
//!   [`expr::lower::LowerError`] composes budget / intern / arena /
//!   side-pool / unknown-op errors with `?`.
//! - [`expr::literals::LiteralPool`] / [`expr::funcs::FuncRegistry`] /
//!   [`expr::slab::OperandSlab`] / [`expr::paths::PathPool`] /
//!   [`expr::types::TypePool`] / [`expr::contexts::ContextPool`] —
//!   the six side-pool carriers consumed by the recursive lowerer.
//!   Path/Func pools dedup structurally; the others are push-only
//!   pending a stable canonical-byte hash.
//!
//! ## What ships in M3d-α (this milestone)
//!
//! - [`schema::Entity`] — entity (table / collection) definition.
//! - [`schema::Field`] / [`schema::FieldType`] — field definitions with
//!   scalar, relation, or computed types.
//! - [`schema::Relation`] / [`schema::RelationKind`] — directed edges
//!   between entities (one-to-one, one-to-many, many-to-many).
//! - [`schema::Lookup`] — indexed access paths (PK, secondary indexes).
//! - [`schema::Policy`] / [`schema::PolicyKind`] — access-control rules.
//! - [`schema::Constraint`] / [`schema::ConstraintKind`] — integrity
//!   constraints (NOT NULL, UNIQUE, PK, FK, CHECK).
//! - `RelationTag`, `LookupTag`, `PolicyTag` in [`dol_cas::handle::tags`]
//!   plus the corresponding `RelationId`, `LookupId`, `PolicyId` type
//!   aliases in [`dol_cas::handle`].
//!
//! ## What lands in M3d-β + M3e (still pending)
//!
//! - **M3d-β**: `SchemaCatalog` (§8.6 catalog struct), constraint
//!   validation passes.
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
pub mod schema;
