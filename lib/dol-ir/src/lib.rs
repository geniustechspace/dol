//! # `dol-ir` — intermediate representation
//!
//! Per `dol-rewrite-plan-v2.md` §8. M3 is the largest milestone in
//! the rewrite and is split across PRs **M3a–M3e**.
//!
//! ## What ships in M3a + M3b + M3c-α + M3c-β + M3c-γ + M3c-δ₁ + M3c-δ₂a + M3c-δ₂b prereq #1 + M3c-δ₂b prereq #2 + M3c-δ₂b prereq #3
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
//! - [`expr::meta`] — tree-DSL leaf metadata: [`expr::meta::OpDef`]
//!   (with [`expr::meta::OpCategory`] and a wire-stable well-known
//!   catalogue) and [`expr::meta::FuncDef`] (with [`expr::meta::Arity`],
//!   [`expr::meta::FuncKind`], and `validate_arity`).
//! - [`expr::frame`] — window / scope frame primitives:
//!   [`expr::frame::FrameUnit`], [`expr::frame::Extent`],
//!   [`expr::frame::Boundary`] (with `unbounded_preceding` /
//!   `unbounded_following` / `preceding(n)` / `following(n)`
//!   helpers), and [`expr::frame::Frame`] with `rows` / `range` /
//!   `groups` constructors.
//! - [`expr::order`] — ordering primitives: [`expr::order::SortDirection`]
//!   (`Asc` default, `Desc`) and [`expr::order::NullsOrder`]
//!   (`First` / `Last` / `Default`), plus [`expr::order::OrderByExpr`]
//!   (the `Expr<'a>`-carrying member of the family).
//! - [`expr::tree`] — the tree DSL: [`expr::tree::Expr`], the user-facing
//!   builder enum with all 14 variants from plan §8.2 (`Ref`, `Param`,
//!   `Lit`, `Seq`, `Map`, `Binary`, `Unary`, `Call`, `Cast`, `Match`,
//!   `If`, `InRange`, `MemberOf`, `Label`, `Wildcard`, `CountAll`,
//!   `Scoped`).
//! - [`expr::context`] — [`expr::context::Context`], the universal
//!   "evaluate inside a scope" descriptor used by `Expr::Scoped`
//!   (carries `partition_by` keys, `order_by` list, optional `frame`).
//!   Plus the fluent builders: [`expr::context::ContextBuilder`]
//!   (builds `Expr::Scoped`) and [`expr::context::ConditionalBuilder`]
//!   (builds `Expr::Match`).
//! - [`expr::lower::lower_path`] — the **single, authorised**
//!   `Path<Name> → Path<StrId>` site (plan §8.4 line 1389), with the
//!   shared [`expr::lower::LowerError`] enum that the upcoming
//!   recursive `lower(Expr)` will reuse unchanged. Charges one
//!   `Budget::node()` per interned segment.
//! - [`expr::literals::LiteralPool`] — typed-arena carrier mapping
//!   [`LiteralId`](dol_cas::handle::LiteralId) to
//!   [`Literal<'static>`](dol_core::literal::Literal). The arena form
//!   of `Expr::Lit(...)` is `ExprNode::lit_ref(LiteralId)` (plan §8.1
//!   line 1095); this pool is its required carrier. Push-only in this
//!   slice — content-addressed dedup needs a stable `Literal` byte
//!   serialisation and is left to a follow-up slice.
//! - [`expr::funcs::FuncRegistry`] — typed-arena carrier mapping
//!   [`FuncId`](dol_cas::handle::FuncId) to
//!   [`FuncDef`](expr::meta::FuncDef). The arena form of
//!   `Expr::Call { func, args }` is `ExprNode::func_ref(FuncId)`
//!   (plan §8.1 line 1097); this registry is its required carrier.
//!   **Name-keyed dedup** — same-named [`FuncDef`]s collapse to a
//!   single id so the surrounding `ExprArena` dedup stays sound.
//!   First-registration-wins on arity/kind conflicts.
//! - [`expr::slab::OperandSlab`] — flat side pool for variadic-arity
//!   operand sequences. `Expr::Seq` / `Expr::Map` / `Expr::Call` /
//!   `Expr::Match` cannot inline their operand lists in the 16-byte
//!   [`ExprNode`](expr::node::ExprNode); the recursive lowerer parks
//!   each sequence here and stores an [`OperandSpan`](expr::slab::OperandSpan)
//!   `(offset, len)` handle in two of the node's `u32` slots. Append-
//!   only, push-only (no interior dedup in this slice — span-level
//!   dedup belongs in `ExprArena::intern_node`).
//!
//! ## What lands in M3c-δ₂b … M3e
//!
//! - **M3c-δ₂b**: the recursive `lower(Expr<'a>) → ExprArena` pass
//!   (§8.4) and `compute_hash` integration (§8.5). All three side-pool
//!   carriers (`LiteralPool` / `FuncRegistry` / `OperandSlab`) are
//!   now in place; the next slice wires them into a recursive lowerer
//!   plus the new `OpFamily` variants (`Seq` / `Map` / `Call` /
//!   `Match`) that consume their handles.
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
