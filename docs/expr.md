# The DOL Expression Layer

This document describes the architecture of `dol-core` and `dol-expr`, the
two crates that together form DOL's *language layer*. Higher crates
(`dol-schema`, `dol-ir`, `dol-pipeline`, etc.) depend on these two and add
no new vocabulary at the expression level.

## Crates

| Crate         | Role                                                                                                                                |
|---------------|-------------------------------------------------------------------------------------------------------------------------------------|
| `dol-core`   | The single source of truth for every type and value in DOL: `DataType`, `Value`, `Literal`, and the supporting primitive types.    |
| `dol-expr`    | The expression engine — the user-facing `tree::Expr<'a>` builder DSL, the 16-byte packed `ExprNode` arena IR it lowers to, and the canonical side-pool node structs. |

Both crates compile in `no_std + alloc` mode (default) and on
`thumbv7em-none-eabihf` for embedded use.

## Two layers

`dol-expr` exposes one expression *representation* (the packed
`ExprNode`) wrapped in two convenient *layers*: a recursive
`tree::Expr<'a>` builder DSL for ergonomic construction and a
flat `ExprArena` of `ExprNode`s for backend rendering. Lowering
flows in one direction — tree → arena.

### `tree::Expr<'a>` — the user-facing builder

`tree::Expr<'a>` is a recursive sum type with a fluent builder API. It is
what end users compose when they write:

```rust,ignore
use dol_expr::tree::{field, param};

let e = field("user.id").eq(param());
```

* **Lifetime-parameterized** so string literals can be borrowed at no cost.
* **Recursive** — `Box<Expr>` for sub-expressions makes building easy but
  is unsuitable for backend rendering (poor cache locality, deep recursion).
* **Ergonomic** — operator overloads (`&`, `|`, `!`), method chains
  (`.eq()`, `.between()`, `.window()`), and dedicated builders for cases,
  windows, and order-by clauses.

### `ExprNode` — the 16-byte packed arena IR

`ExprNode` is a `#[repr(C)]`, `bytemuck::Pod` POD struct stored in an
`ExprArena`:

* **`size_of::<ExprNode>() == 16`** is asserted in `xtask size` and in
  unit tests inside `dol-expr`. The layout is `op (u8) + flags (u8) +
  aux (u16) + a (u32) + b (u32) + c (u32)`. Opcodes that need more
  than three operand handles (Func args, Window order-by, Case branches,
  array literals, …) spill into typed side pools on `ExprArena`
  (`fields`, `funcs`, `cases`, `windows`, `queries`, `inserts`,
  `updates`, `deletes`, `upserts`, `in_lists`, `obj_lits`,
  `array_lits`).
* **Indices, not pointers** — every cross-reference is a typed `*Id` (a
  `u32` `NonZeroU32`). This eliminates allocator pressure, makes the
  IR trivially serializable, and keeps cache lines hot (≈2× the
  density of the previous 32-byte variant enum).
* **Typed accessors** — `node.as_bin()`, `node.as_field()`, … return
  `Option<…>` so a malformed/unknown opcode surfaces cleanly. Backends
  must use these helpers; reaching into raw `a`/`b`/`c` defeats the
  centralised `BinOp::try_from_u16` / `UnaryOp::try_from_u16`
  validation.
* **Backend-friendly** — backends (SQL, REST, KV, …) traverse
  `ExprArena` directly, never the tree.

## Lowering: tree → arena

The `dol-expr::lower` module bridges `tree::Expr<'static>` to
`(ExprArena, NodeId)`. Lowering is the single point that transforms the
ergonomic tree into the cache-friendly IR.

Invariants:

1. **Pure**: lowering does not allocate side-effects beyond the arena and
   interner. No global state.
2. **Stable**: identical input trees yield identical arenas (modulo the
   order in which independent strings are interned, which is determined
   by traversal order — itself stable).
3. **One-way**: there is no arena-to-tree lowering. The arena is the
   serialization boundary; once you cross it, you stay in `ExprNode`
   territory.

## Identifier model

Every cross-reference inside the arena is one of these typed indices:

| Type        | Pool                          | Notes                                  |
|-------------|-------------------------------|----------------------------------------|
| `NodeId`    | `ExprArena::nodes`            | All inline expression nodes.           |
| `StrId`     | `Interner::strings`           | Stable for the lifetime of the arena.  |
| `LiteralId` | `ExprArena::lits`             | Boxed for size.                        |
| `FieldId`   | `ExprArena::fields`           | Field traversal payload.               |
| `FuncId`    | `ExprArena::funcs`            | Function call payload.                 |
| `WindowId`  | `ExprArena::windows`          | Window-function payload.               |
| `CaseId`    | `ExprArena::cases`            | `CASE WHEN … THEN … ELSE …` payload.   |
| `InListId`  | `ExprArena::in_lists`         | `expr IN (…)` payload.                 |
| `ObjLitId`  | `ExprArena::obj_lits`         | Object-literal payload.                |
| `QueryId`   | `ExprArena::queries`          | Sub-query payload.                     |
| `InsertId`  | `ExprArena::inserts`          | INSERT payload.                        |
| `UpdateId`  | `ExprArena::updates`          | UPDATE payload.                        |
| `DeleteId`  | `ExprArena::deletes`          | DELETE payload.                        |
| `UpsertId`  | `ExprArena::upserts`          | UPSERT payload.                        |

All `*Id` types are 32-bit. They are valid only within the arena that
produced them; mixing IDs across arenas is a logic bug — the type system
catches accidental cross-pool aliasing because each pool has its own
distinctly-tagged ID type.

## The interner

`dol_expr::Interner` deduplicates strings to a `StrId`:

* Backed by `hashbrown::HashMap<Arc<str>, StrId>` and a `Vec<Arc<str>>`.
* Each unique string is allocated once and shared between the lookup map
  and the id-lookup table.
* Determinism: the canonical wire form is the **`Vec<Arc<str>>` in
  insertion order**. The hand-rolled serde codec writes only that vector;
  the map is reconstructed on deserialization. Postcard / JSON / any
  serde-compatible format yields a byte-stable representation provided
  upstream code interns strings in the same order.

## `DataType` / `Value` contract

`DataType` describes what is **expected** at a position. `Value` is what
**arrives** there.

* `DataType::accepts(&value) -> Result<(), TypeError>` is the conformance
  bridge.
* Validation errors are structured (`TypeError`), never panics, never
  strings.
* All construction is via `try_*` constructors that surface the same
  `TypeError` variants.

## Cargo features

| Crate       | Feature   | Default | Effect                                                                                                |
|-------------|-----------|---------|-------------------------------------------------------------------------------------------------------|
| `dol-core` | `std`     | ✓       | Enables `datetime::today() / now() / now_tz()` (need `SystemTime`).                                   |
| `dol-core` | `serde`   | ✓       | `Serialize` / `Deserialize` for every public type.                                                    |
| `dol-expr`  | `std`     | —       | Forwards to `dol-core/std`. The crate itself is otherwise `no_std + alloc`.                          |
| `dol-expr`  | `serde`   | —       | `Serialize` / `Deserialize` for every AST/arena type and `Interner`.                                  |

## Tests and gates

* Serde round-trip is gated in CI for both crates
  (`{lib,tools}/*/tests/serde_roundtrip.rs`).
* `xtask size` asserts `size_of::<ExprNode> == 16`,
  `size_of::<Value> == 24`, and `size_of::<Literal<'static>> == 32`.
* `xtask nostd` and the `cross-compile` CI job run
  `cargo check --no-default-features` for both crates, and additionally
  `--target thumbv7em-none-eabihf` for the cross-compile job.

## See also

* [`docs/STABILITY.md`](./STABILITY.md) — public-API and wire-format
  stability policy.
* `lib/core/README.md` and `lib/expr/README.md` — quick
  reference for each crate's public surface.
