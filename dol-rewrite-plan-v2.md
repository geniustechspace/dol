# DOL — Complete Project Rewrite Plan (v2)

> **Version:** 2.0 — Incorporates `Name`, `PathSegment`, `Path<N>`, and full `Expr<'a>` AST  
> **Edition:** Rust 2024 · MSRV 1.85 · `no_std + alloc` capable throughout  
> **Status:** Pre-implementation — architectural specification

---

## Table of Contents

1. [Rewrite Rationale](#1-rewrite-rationale)
2. [Architectural Principles](#2-architectural-principles)
3. [Crate Inventory](#3-crate-inventory)
4. [Dependency DAG](#4-dependency-dag)
5. [Phase 0 — Workspace Scaffolding](#5-phase-0--workspace-scaffolding)
6. [Phase 1 — `dol-core`](#6-phase-1--dol-core)
7. [Phase 2 — `dol-cas`](#7-phase-2--dol-cas)
8. [Phase 3 — `dol-ir`](#8-phase-3--dol-ir)
9. [Phase 4 — `dol-wire`](#9-phase-4--dol-wire)
10. [Phase 5 — `dol-query`](#10-phase-5--dol-query)
11. [Phase 6 — Umbrella + First Backend](#11-phase-6--dol-umbrella--first-backend)
12. [Cross-Cutting Concerns](#12-cross-cutting-concerns)
13. [Testing Strategy](#13-testing-strategy)
14. [CI Pipeline](#14-ci-pipeline)
15. [Risk Register](#15-risk-register)
16. [Milestone Summary](#16-milestone-summary)

---

## 1. Rewrite Rationale

The prototype established correct domain vocabulary and the right invariants
(budget-gated traversal, serialize-only wire policy, single hash chokepoint,
no unsafe). What it got wrong is crate topology, identity architecture, and
the string/path layer.

| Problem | Impact |
|---|---|
| 9 `lib/` crates with tightly-coupled churn | Every internal refactor is a multi-crate coordinated bump |
| `dol-expr`, `dol-schema`, `dol-command` have no independent consumers | Crate boundaries where module boundaries suffice |
| BLAKE3 on every `intern()` call | 3–8× unnecessary CPU on the hot path |
| 32-bit `StrId` truncated from BLAKE3 | Birthday collision boundary at ~65 K unique strings |
| No `Cid`/`Gid` distinction | Cross-process stability mixed with in-process dedup |
| `Interner` lives in `dol-expr` | Identity is not an expression concern |
| No `Name` type | Static and owned strings share no common type; `&'static str` leaks everywhere |
| No `PathSegment` abstraction | Paths are either all-string or all-opaque with no bridge |
| `Path` is not generic over its segment type | Cannot move a path from tree DSL (inline strings) to arena (interned IDs) |
| No `Config` — limits are ad-hoc constants | Cannot tune for IoT vs cloud without recompilation |
| Backends directory empty | No feedback loop on whether `Operation`/`Backend` are correct |

The rewrite consolidates to **5 library crates + 1 umbrella**, introduces a
dedicated content-addressing layer (`dol-cas`), replaces the interner with a
general `StringPool`, promotes `Name` and `Path<N: PathSegment>` into
`dol-core` as first-class types, and introduces a `Config` system that drives
all resource limits and profile selection.

---

## 2. Architectural Principles

These are non-negotiable. Every design decision below is derived from them.

**P1 — Dependency direction is the architecture.**
The DAG `core → cas → ir → wire, query → dol` is the skeleton. Nothing flows
backwards. No circular dependencies, ever. A crate's position in the DAG
determines what it is allowed to know.

**P2 — Crate boundaries justify themselves.**
A crate split is justified only when it provides independent versioning,
optional dependency weight, or a different stability contract. Module
boundaries handle everything else.

**P3 — Local identity is free; stable identity is opt-in.**
`Lid<Tag>` (sequential, arena-local) costs nothing at runtime. `Cid<Tag>`
(content-addressed, cross-process stable) is computed lazily, only when
a caller explicitly needs cross-process identity.

**P4 — Two string modes, one bridge.**
`Name` is the inline string type — static or owned, no pool, no ID.
`Lid<StrTag>` is the interned string type — 4 bytes, pool-backed.
`PathSegment` is the trait that lets `Path<N>` work with either without
duplication. Lowering from tree to arena is where `Path<Name>` becomes
`Path<Lid<StrTag>>`.

**P5 — Budget gates every recursive public function.**
Every `walk_*`, `visit_*`, `lower_*`, `decode_*`, `content_hash_*` takes
`&mut Budget`. No exceptions. CI enforces this via `xtask budget-gate`.

**P6 — Serde in, never Deserialize.**
In-memory IR, schema, and expression types are `Serialize`-only.
All wire-in goes through `dol_wire::Decode`, which validates IDs and
threads `&mut Budget` at every descent.

**P7 — The hash module is the only blake3 caller.**
`dol_core::hash` exports four functions. No other crate calls `blake3` or
`xxhash_rust` directly.

**P8 — No panic in library code except documented design-time errors.**
Design-time errors (arena overflow at 4G entries, impossible opcode) are
documented with `// design-time error: <reason>` and paired with fallible
siblings. All other paths are `Result` or `Option`.

**P9 — `#![forbid(unsafe_code)]` workspace-wide.**
No exceptions.

**P10 — `ExprNode` is exactly 16 bytes.**
Asserted in unit tests and `xtask size-check`. Side pools exist precisely
to protect it.

**P11 — Config is the single source of tuning.**
All resource limits, shard counts, hash strategies, and profile selections
live in `dol_core::config::Config`. Nothing is a magic constant.

---

## 3. Crate Inventory

### 3.1 `dol-core` — Pure Foundation

**Owns:** Primitive types, the `Name` string type, the `PathSegment` trait,
the generic `Path<N>`, configuration, budget, hash chokepoints,
span/diagnostic.

**Does not own:** Any ID types, arenas, interners, IR, wire logic. The
`PathSegment` impl for `Lid<StrTag>` lives in `dol-cas`, not here — `dol-core`
cannot know about `dol-cas`.

**`no_std + alloc`:** Yes, always. `SmallVec` use in `Path` is gated on
`feature = "alloc"`; bare-metal targets use a fixed inline array with a
length counter.

**Key invariant:** Zero workspace dependencies. Every other crate depends on
this; it depends on nothing in the workspace.

```
dol-core/
  src/
    lib.rs
    config/
      mod.rs          Config, Profile, HashStrategy
      budget.rs       BudgetConfig
      pool.rs         PoolConfig
      hash.rs         HashConfig
    budget.rs         Budget, BudgetExceeded
    hash.rs           fast64, fast64_seeded, fast128, content128, content256
    strings/
      mod.rs
      name.rs         Name (Static | Owned), all trait impls, Serialize-only
    path/
      mod.rs          Path<N: PathSegment>, SmallVec field chain
      segment.rs      PathSegment trait, impl for Name (Resolver = ())
    types/
      mod.rs
      data_type.rs    DataType enum
      value.rs        Value, Literal<'a>
      numeric.rs      Decimal, fixed-point helpers
      temporal.rs     Date, Time, DateTime, Duration
      network.rs      IpAddr, MacAddr
    span.rs           Span, ByteOffset, SourceId
    diagnostic.rs     Diagnostic, Severity, DiagnosticCode, DiagnosticBuilder
```

### 3.2 `dol-cas` — Content Addressing System

**Owns:** All handle types (`Lid`, `Cid`, `Gid`), generic pool, string pool,
content index, and the `PathSegment` impl for `Lid<StrTag>`.

**Does not own:** Any domain IR, schema types, or wire logic.

**`no_std + alloc`:** Yes. `StaticPool` and `StaticStringPool` additionally
work without alloc (bare metal).

**Key invariant:** `StringPool` fully replaces the old `Interner`.
`StrId = Lid<StrTag>` is the new interned string handle.

```
dol-cas/
  src/
    lib.rs
    handle/
      mod.rs
      lid.rs            Lid<Tag>: NonZeroU32, phantom, all traits
      cid.rs            Cid<Tag>: [u8;16], content-addressed
      gid.rs            Gid<Tag>: [u8;32], cryptographic
      tags.rs           Tag marker types: StrTag, NodeTag, FieldTag, …
    pool/
      mod.rs            ArenaStorage trait
      dyn_pool.rs       DynPool<T>  (Vec-backed, alloc)
      static_pool.rs    StaticPool<T, const CAP: usize>  (no alloc)
    string_pool/
      mod.rs            StringPool, InternError
      shard.rs          ShardInner, seeded xxHash3 lookup
      static_str.rs     StaticStringPool<const CAP: usize>
      segment_impl.rs   PathSegment for Lid<StrTag> (Resolver = StringPool)
    content_index/
      mod.rs            ContentIndex, MerkleNode
      walk.rs           bottom-up hash walk, budget-gated
```

### 3.3 `dol-ir` — Intermediate Representation

**Owns:** Expression arena, tree DSL, schema catalog, operations, programs,
backend trait. Optionally stream and pipeline IRs.

**Does not own:** Wire encoding, query builder DSL, concrete backends.

**`no_std + alloc`:** Yes for the base. Stream/pipeline features require
`alloc` but not `std`.

**Feature flags:**

| Flag | Enables |
|---|---|
| `stream` | `dol_ir::stream` module |
| `pipeline` | `dol_ir::pipeline` module |
| `content-index` | `ContentIndex` integration on `ExprArena` |

```
dol-ir/
  src/
    lib.rs
    expr/
      mod.rs
      node.rs         ExprNode (16B POD), opcode tables, side pools
      arena.rs        ExprArena, alloc_* constructors, intern_node
      ops.rs          BinOp, UnaryOp, AggOp, CastOp — append-only tables
      tree.rs         Expr<'a> full AST (all variants — see §8.2)
      lower.rs        lower(&Expr, &mut ExprArena, &mut StringPool, &mut Budget)
                      Path<Name> → Path<Lid<StrTag>> happens here
      context.rs      Context<'a>, ContextBuilder, ConditionalBuilder
      order.rs        OrderByExpr<'a>, SortDirection, NullsOrder
      func/
        mod.rs
        meta.rs       FuncDef, FuncKind, ReturnType
      op/
        mod.rs        UnaryOp enum
        meta.rs       OpDef, precedence, associativity, dialect hints
      side/
        field.rs      FieldNode, FieldId
        func.rs       FuncNode (resolved, post-lower)
        composite.rs  CompositeNode, CompositeId
        stmt.rs       InsertNode, UpdateNode, DeleteNode, UpsertNode
    schema/
      mod.rs
      entity.rs       Entity, EntityId
      field.rs        Field, FieldType, FieldId
      constraint.rs   Constraint, ConstraintKind
      relation.rs     Relation, RelationKind, RelationId
      lookup.rs       Lookup, LookupId
      policy.rs       Policy, PolicyKind
      catalog.rs      SchemaCatalog, SchemaRef
      type_body.rs    TypeBody (sum / product / newtype)
    command/
      mod.rs
      operation.rs    Operation enum (all variants)
      program.rs      Program, ProgramId
      capability.rs   Capabilities, CapabilitySet
      backend.rs      Backend trait
      builders/
        ddl.rs        DdlBuilder
        acl.rs        AclBuilder
        tx.rs         TxBuilder
        storage.rs    StorageBuilder
    stream/           #[cfg(feature = "stream")]
      mod.rs
      types.rs        StreamDef, WindowSpec, WatermarkPolicy
      ir.rs           StreamOperation, StreamProgram
    pipeline/         #[cfg(feature = "pipeline")]
      mod.rs
      graph.rs        PipelineGraph, Node, Edge
      ir.rs           PipelineOperation
```

### 3.4 `dol-wire` — Wire Protocol

**Owns:** Envelope format, all codec implementations, the `Decode` trait and
all budget-gated implementations.

**Key invariant:** `Decode` is the **only** path by which external bytes
become in-memory IR.

```
dol-wire/
  src/
    lib.rs
    envelope.rs       WireEnvelope, WireHeader, content_id verification
    decode/
      mod.rs          Decode trait
      lid.rs          Decode for Lid<Tag>
      cid.rs          Decode for Cid<Tag>
      program.rs      Decode for Program
      operation.rs    Decode for Operation (all variants)
      schema.rs       Decode for SchemaCatalog
      expr.rs         Decode for ExprArena
    codec/
      mod.rs          Encode trait
      postcard.rs     postcard encode/decode adapter
      json.rs         JSON encode/decode adapter
    error.rs          WireError, DecodeError, EnvelopeError
```

### 3.5 `dol-query` — Query Builder DSL

**Owns:** Fluent builder API that produces a `Program` via `try_build`.
Operates entirely on `Path<Name>` and `Expr<'a>` with `Name` segments; the
lowering inside `try_build` interns all names into the `StringPool`.

```
dol-query/
  src/
    lib.rs
    query.rs          Query entry point
    builders/
      select.rs       SelectBuilder → Operation::Select
      insert.rs       InsertBuilder → Operation::Insert
      update.rs       UpdateBuilder → Operation::Update
      delete.rs       DeleteBuilder → Operation::Delete
      upsert.rs       UpsertBuilder → Operation::Upsert
    context.rs        QueryContext (holds SchemaRef + StringPool ref + Config ref)
    error.rs          BuildError, FieldNotFound, TypeMismatch, BudgetExceeded
    validate.rs       pre-build schema validation pass
```

### 3.6 `dol` — Umbrella Facade

```
dol/
  src/
    lib.rs            conditional re-exports per feature preset
    context.rs        Dol::builder().with_config(cfg).build()
    prelude.rs        wildcard import surface for application developers
```

**Feature presets:**

| Preset | Includes |
|---|---|
| `core` | `dol-core` + `dol-cas` only |
| `full` | Everything, `std`, stream, pipeline |
| `iot-min` | `dol-core` + `dol-cas` + `dol-ir` (base), no_std, static pools |

---

## 4. Dependency DAG

```
                    ┌─────────────────────────────────┐
                    │           dol-core               │
                    │  config · budget · hash          │
                    │  Name · PathSegment · Path<N>    │
                    │  DataType · Literal · Span       │
                    │  Diagnostic                      │
                    └──────────────┬──────────────────┘
                                   │
                    ┌──────────────▼──────────────────┐
                    │           dol-cas                │
                    │  Lid · Cid · Gid                 │
                    │  Pool · StringPool               │
                    │  ContentIndex                    │
                    │  PathSegment for Lid<StrTag>     │
                    └──────────────┬──────────────────┘
                                   │
                    ┌──────────────▼──────────────────┐
                    │           dol-ir                 │
                    │  Expr<'a> · ExprArena            │
                    │  Schema · Operation              │
                    │  Program · Backend               │
                    │  [stream] [pipeline]             │
                    └──────┬──────────────┬───────────┘
                           │              │
          ┌────────────────▼──┐    ┌──────▼─────────────┐
          │     dol-wire       │    │     dol-query       │
          │  Envelope · Decode │    │  DSL → Program      │
          └────────────────┬──┘    └──────┬──────────────┘
                           │              │
                    ┌──────▼──────────────▼──────────────┐
                    │                dol                  │
                    │  umbrella · presets · Dol context   │
                    └─────────────────────────────────────┘

tools/check  ──► dol-ir
tools/fmt    ──► dol-ir
backends/*   ──► dol-ir (+ optionally dol-wire)
xtask        ──► workspace metadata only
```

**Rules enforced by CI (`xtask dag-check`):**
- Nothing in `lib/` depends on `tools/` or `backends/`
- `dol-core` has zero workspace dependencies
- `dol-cas` depends only on `dol-core`
- `dol-ir` depends only on `dol-cas`, `dol-core`
- `dol-wire` depends only on `dol-ir`, `dol-core`
- `dol-query` depends only on `dol-ir`, `dol-core`
- Backends depend only on `dol-ir` (and optionally `dol-wire`)

---

## 5. Phase 0 — Workspace Scaffolding

**Goal:** Green CI on empty crates. Every subsequent phase has instant
feedback from the moment the first line is written.

**Duration estimate:** 1–2 days.

### 5.1 Workspace `Cargo.toml`

```toml
[workspace]
members = [
  "lib/dol-core",
  "lib/dol-cas",
  "lib/dol-ir",
  "lib/dol-wire",
  "lib/dol-query",
  "dol",
  "tools/check",
  "tools/fmt",
  "xtask",
]
resolver = "2"

[workspace.package]
edition      = "2024"
rust-version = "1.85"
license      = "MIT OR Apache-2.0"

[workspace.dependencies]
# Hash layer — only dol-core may depend on these directly
blake3      = { version = "1",   default-features = false }
xxhash-rust = { version = "0.8", features = ["xxh3"] }

# Compact inline vec — only dol-core (Path field chain)
# Gated on feature = "alloc"; bare-metal uses inline array fallback
smallvec = { version = "1", default-features = false, features = ["const_generics"] }

# Serialization — only dol-wire may use full serde derive
serde    = { version = "1", default-features = false, features = ["derive"] }
postcard = { version = "1", default-features = false }

# Testing
proptest  = "1"
criterion = "0.5"
insta     = "1"

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all      = "warn"
pedantic = "warn"
```

### 5.2 `xtask` Skeleton

Implement as empty stubs that exit 0. Fill in logic as the crates they
check come to exist.

| Task | What it checks |
|---|---|
| `xtask budget-gate` | Every public `walk_*` / `visit_*` / `lower_*` / `decode_*` / `content_hash_*` takes `&mut Budget` |
| `xtask size-check` | `size_of::<ExprNode>() == 16`, `size_of::<Option<Lid<_>>>() == 4` |
| `xtask dag-check` | Dependency edges match rules in §4 |
| `xtask no-std-check` | Builds `dol-core`, `dol-cas`, `dol-ir` for `thumbv7em-none-eabihf` |
| `xtask size-report` | Per-crate `.rlib` sizes, binary size for IoT demo |

### 5.3 CI Matrix

```yaml
jobs:
  test:
    matrix:
      os:   [ubuntu-latest, macos-latest, windows-latest]
      rust: [stable, 1.85]      # MSRV gate on every push

  no-std:
    target: thumbv7em-none-eabihf
    features: iot-min

  miri:
    # dol-core and dol-cas on every push; dol-ir when ExprArena stable
    toolchain: nightly

  fuzz:
    # dol-wire decode targets (libFuzzer)
    toolchain: nightly

  xtask:
    steps: [budget-gate, size-check, dag-check]
```

### 5.4 Deliverable

Workspace compiles cleanly, all 6 crates declared, CI green on empty
`lib.rs` files, all `xtask` stubs run without error.

---

## 6. Phase 1 — `dol-core`

**Goal:** A frozen, zero-dependency foundation that compiles on bare metal.
This crate's public API should be considered nearly stable once published.

**Duration estimate:** 4–6 days (up from 3–5 due to `Name` / `Path` work).

**External dependencies:** `blake3`, `xxhash-rust` (hash module only),
`smallvec` (Path field chain, alloc-gated), `serde` (Serialize only).

### 6.1 `config` Module — First

Everything else is parameterized by it. Goes in before any type that uses
resource limits or profile selection.

```rust
// config/mod.rs
pub struct Config {
    pub budget:  BudgetConfig,
    pub pool:    PoolConfig,
    pub hash:    HashConfig,
    pub profile: Profile,
}

impl Config {
    pub fn standard() -> Self { /* std defaults  */ }
    pub fn embedded() -> Self { /* no_std + alloc */ }
    pub fn iot_min()  -> Self { /* no_std, static pools */ }
}

// config/budget.rs
pub struct BudgetConfig {
    pub max_depth: u32,   // default: 512
    pub max_nodes: u32,   // default: 1_000_000
    pub max_bytes: u64,   // default: 64 MiB
}

// config/pool.rs
pub struct PoolConfig {
    pub shard_count:   usize,  // default: 64, must be power-of-two
    pub hash_seed:     u64,    // DoS-protection seed for xxHash3
    pub initial_bytes: usize,  // bump pre-alloc per shard; default: 4096
}

// config/hash.rs
pub enum HashStrategy {
    /// xxHash3-64 seeded. Fast. Non-cryptographic.
    /// Cids computed this way are NOT cross-process stable unless all
    /// processes use the same seed.
    Fast { seed: u64 },

    /// BLAKE3 truncated to 128 bits. Default.
    /// Cross-process stable. ~1 GB/s on ARM Cortex-M4.
    Crypto,

    /// Full BLAKE3 256-bit output.
    /// For signed manifests and tamper-evident wire envelopes.
    CryptoFull,
}
```

### 6.2 `budget` Module

```rust
pub struct Budget {
    pub depth: u32,
    pub nodes: u32,
    pub bytes: u64,
}

impl Budget {
    pub fn from_config(cfg: &BudgetConfig) -> Self;

    #[inline]
    pub fn depth(&mut self) -> Result<(), BudgetExceeded> {
        self.depth = self.depth.checked_sub(1).ok_or(BudgetExceeded::Depth)?;
        Ok(())
    }

    #[inline]
    pub fn node(&mut self) -> Result<(), BudgetExceeded>;

    #[inline]
    pub fn bytes(&mut self, n: u64) -> Result<(), BudgetExceeded>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetExceeded { Depth, Nodes, Bytes }
```

### 6.3 `hash` Module

The single place in the entire workspace that calls `blake3` or
`xxhash_rust` directly.

```rust
/// Fast non-cryptographic hash. In-process dedup only.
/// Do NOT use as a cross-process stable key.
pub fn fast64(bytes: &[u8]) -> u64;
pub fn fast64_seeded(bytes: &[u8], seed: u64) -> u64;
pub fn fast128(bytes: &[u8]) -> u128;

/// BLAKE3 truncated to 128 bits. Cross-process stable.
/// Suitable for Cid computation.
pub fn content128(bytes: &[u8]) -> [u8; 16];

/// Full BLAKE3 256 bits. For Gid and signed manifests.
pub fn content256(bytes: &[u8]) -> [u8; 32];
```

### 6.4 `strings` Module — `Name`

`Name` is the inline string type used across all user-facing APIs, tree DSL
construction, and any context where strings are short-lived or static.
It is distinct from `Lid<StrTag>`, which is the interned, pool-backed handle.

```rust
// strings/name.rs

/// A compact symbolic name with two storage modes.
///
/// - `Static` for compile-time-known names: zero allocation.
/// - `Owned`  for runtime names: one heap allocation via `Box<str>`.
///
/// All equality, ordering, hashing, and serialization operate on
/// string content, not storage variant.
///
/// Does not implement `Deserialize`. Wire-in goes through `dol_wire::Decode`.
#[derive(Debug, Clone)]
pub enum Name {
    Static(&'static str),
    Owned(Box<str>),
}

impl Name {
    pub fn owned(s: impl Into<Box<str>>) -> Self;
    pub fn as_str(&self) -> &str;
    pub const fn is_static(&self) -> bool;
    pub const fn is_owned(&self) -> bool;

    /// Convenience constructor for use in APIs that accept any string-like
    /// value. Prefer `Name::Static("literal")` for compile-time-known names.
    #[inline]
    pub fn new(s: impl Into<Name>) -> Self { s.into() }
}

// From<&'static str> → Name::Static (zero allocation)
// From<String>       → Name::Owned
// From<Box<str>>     → Name::Owned

// PartialEq, Eq, Ord, Hash: all by as_str() content
// Display:  writes as_str()
// Serialize: serializer.serialize_str(self.as_str())
// Deserialize: intentionally absent
```

### 6.5 `path` Module — `PathSegment` Trait and `Path<N>`

This is the bridge between the two string modes. `Path<Name>` is used in
tree DSL and query builder APIs (inline strings, no resolver needed).
`Path<Lid<StrTag>>` is used in the arena and backend layer (compact IDs,
`StringPool` resolver). Lowering converts one to the other.

```rust
// path/segment.rs

/// A value that can be stored as a segment inside a [`Path`].
///
/// # Resolver contract
///
/// - Inline string-backed segments (e.g. `Name`) use `Resolver = ()`.
/// - Interned ID segments (e.g. `Lid<StrTag>`) use the issuing pool type
///   as their resolver. This prevents accidentally resolving an interned
///   path without the pool needed to make its IDs meaningful.
pub trait PathSegment: Clone + Eq + Hash + fmt::Debug {
    /// Context required to resolve this segment to a string.
    /// Use `()` for inline segments.
    type Resolver: ?Sized;

    /// Resolves this segment to a string slice.
    /// The lifetime ties the output to both `self` and `resolver`.
    fn resolve<'a>(&'a self, resolver: &'a Self::Resolver) -> &'a str;

    /// Returns the cross-process stable content address for this segment,
    /// if available.
    ///
    /// `Name` segments return `None` — they carry no stable identity.
    /// `Lid<StrTag>` segments return the lazily-computed BLAKE3 address
    /// from the `StringPool`.
    ///
    /// Default: always `None`.
    fn content_id(&self, _resolver: &Self::Resolver) -> Option<[u8; 16]> {
        None
    }
}

/// `Name` segments resolve without context.
impl PathSegment for Name {
    type Resolver = ();

    #[inline]
    fn resolve<'a>(&'a self, _: &'a ()) -> &'a str {
        self.as_str()
    }

    // content_id: default None — Name carries no stable identity
}
```

```rust
// path/mod.rs

/// A structured, store-neutral path to a target and optional nested fields.
///
/// | Part        | Meaning                                    | Required |
/// |-------------|--------------------------------------------|----------|
/// | `namespace` | Optional scope / authority token           | no       |
/// | `target`    | Addressable resource token                 | yes      |
/// | `field`     | Optional chain into the addressed resource | no       |
///
/// The default segment type `N = Name` gives inline, resolver-free paths
/// suitable for query construction. Lowering into the arena produces
/// `Path<Lid<StrTag>>` with compact 4-byte segments.
///
/// # Field chain storage
///
/// Under `feature = "alloc"`: `SmallVec<[N; 2]>` — inline for 0–2 segments,
/// heap-allocated beyond that. Most paths have zero or one field segment.
///
/// Under bare-metal (`no alloc`): `[Option<N>; 4]` with a `u8` length counter.
/// Paths with more than 4 field segments are a `PathError::TooDeep` at
/// construction time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Path<N: PathSegment = Name> {
    namespace: Option<N>,
    target:    N,

    #[cfg(feature = "alloc")]
    field: SmallVec<[N; 2]>,

    #[cfg(not(feature = "alloc"))]
    field: [Option<N>; 4],
    #[cfg(not(feature = "alloc"))]
    field_len: u8,
}

impl<N: PathSegment> Path<N> {
    pub fn from_segment(target: N) -> Self;
    pub fn namespace(mut self, ns: impl Into<N>) -> Self;
    pub fn get(mut self, segment: impl Into<N>) -> Self;

    pub fn has_namespace(&self) -> bool;
    pub fn has_fields(&self)    -> bool;
    pub fn field_len(&self)     -> usize;

    pub fn namespace_segment(&self)  -> Option<&N>;
    pub fn target_segment(&self)     -> &N;
    pub fn field_segments(&self)     -> &[N];

    pub fn resolve_namespace<'a>(&'a self, r: &'a N::Resolver) -> Option<&'a str>;
    pub fn resolve_target<'a>  (&'a self, r: &'a N::Resolver) -> &'a str;
    pub fn resolve_fields<'a>  (&'a self, r: &'a N::Resolver)
        -> impl Iterator<Item = &'a str> + 'a;
}

// String-specific helpers, only when N = Name (Resolver = ())
impl Path<Name> {
    pub fn new(target: impl Into<Name>) -> Self;
    pub fn namespace_str(&self) -> Option<&str>;
    pub fn target_str(&self)    -> &str;
    pub fn field_strs(&self)    -> impl Iterator<Item = &str>;
    pub fn field_single(&self)  -> Option<&str>;
}
```

### 6.6 `types` Module

```rust
// types/data_type.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub enum DataType {
    Bool,
    Int8, Int16, Int32, Int64,
    UInt8, UInt16, UInt32, UInt64,
    Float32, Float64,
    Decimal { precision: u8, scale: u8 },
    Text,
    Bytes,
    Uuid,
    Date, Time, DateTime, Duration,
    IpAddr, MacAddr,
    Json,
    Array(Box<DataType>),
    Map { key: Box<DataType>, value: Box<DataType> },
    Optional(Box<DataType>),
}

// types/value.rs
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Literal<'a> {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    Decimal(Decimal),
    Str(&'a str),        // zero-copy during AST construction
    StrOwned(Box<str>),  // after .into_owned() / lowering
    Bytes(&'a [u8]),
    BytesOwned(Box<[u8]>),
    Uuid([u8; 16]),
    Date(Date),
    DateTime(DateTime),
    IpAddr(IpAddr),
}

impl<'a> Literal<'a> {
    // Convenience constructors
    pub fn int64(n: i64)      -> Self { Self::Int(n) }
    pub fn str(s: &'a str)    -> Self { Self::Str(s) }
    pub fn bool(b: bool)      -> Self { Self::Bool(b) }

    /// Erases the lifetime, converting borrowed variants to owned.
    pub fn into_owned(self) -> Literal<'static>;
}
```

### 6.7 `span` and `diagnostic` Modules

```rust
// span.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Span { pub start: u32, pub end: u32 }

// diagnostic.rs
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code:     DiagnosticCode,
    pub message:  alloc::string::String,
    pub span:     Option<Span>,
    pub notes:    alloc::vec::Vec<alloc::string::String>,
}
```

### 6.8 Phase 1 Tests

- `Config::standard()` produces valid non-zero budget/pool values
- `Budget` returns `BudgetExceeded` variants correctly
- `fast64` and `content128`: same input → same output; different input → different output with overwhelming probability
- `Name::Static("x") == Name::Owned("x".into())` — content equality
- `Path::new("orders").get("total").field_single() == Some("total")`
- `Path::new("orders").get("a").get("b").field_single() == None`
- `Path<Name>` resolves without a resolver: `p.resolve_target(&()) == "orders"`
- `size_of::<Option<Path<Name>>>()` documented (soft, not hard)
- `Path` compiles for `thumbv7em-none-eabihf` (xtask no-std-check, bare-metal inline array path)

---

## 7. Phase 2 — `dol-cas`

**Goal:** Identity and pooling layer. `StringPool` fully replaces the old
`Interner`. `PathSegment for Lid<StrTag>` closes the two-mode path bridge.

**Duration estimate:** 4–6 days.

### 7.1 Handle Types

```rust
// handle/lid.rs
#[repr(transparent)]
pub struct Lid<Tag: ?Sized> {
    raw:     NonZeroU32,
    _marker: PhantomData<fn() -> Tag>,
}

impl<Tag: ?Sized> Lid<Tag> {
    pub fn from_index(i: usize) -> Option<Self>;
    pub fn from_u32(n: u32)     -> Option<Self>;
    pub fn index(self) -> usize { self.raw.get() as usize - 1 }
    pub fn get(self)   -> u32   { self.raw.get() }
}

// Serialize as bare u32. No Deserialize — ever.
// Clone, Copy, Eq, Ord, Hash: hand-written, no Tag bound.

// handle/cid.rs
#[repr(C)]
pub struct Cid<Tag: ?Sized> {
    raw:     [u8; 16],
    _marker: PhantomData<fn() -> Tag>,
}

// handle/gid.rs — same shape, [u8; 32]
```

**Invariant test:** `assert_eq!(size_of::<Option<Lid<()>>>(), 4)`

### 7.2 Tag Types and Type Aliases

```rust
// handle/tags.rs
pub enum StrTag     {}
pub enum NodeTag    {}
pub enum FieldTag   {}
pub enum EntityTag  {}
pub enum LiteralTag {}
pub enum FuncTag    {}
pub enum SchemaTag  {}
pub enum ProgramTag {}

pub type StrId      = Lid<StrTag>;
pub type NodeId     = Lid<NodeTag>;
pub type FieldId    = Lid<FieldTag>;
pub type EntityId   = Lid<EntityTag>;
pub type LiteralId  = Lid<LiteralTag>;
pub type FuncId     = Lid<FuncTag>;

pub type StrCid     = Cid<StrTag>;
pub type SchemaCid  = Cid<SchemaTag>;
pub type ProgramCid = Cid<ProgramTag>;
pub type ProgramGid = Gid<ProgramTag>;
```

### 7.3 `Pool<T>` — Generic Arena

```rust
pub trait ArenaStorage<T> {
    fn push(&mut self, item: T) -> Option<usize>;
    fn get(&self, index: usize)     -> Option<&T>;
    fn get_mut(&mut self, index: usize) -> Option<&mut T>;
    fn len(&self) -> usize;
}

// DynPool<T>: Vec-backed, requires alloc
// StaticPool<T, const CAP: usize>: fixed-size, no alloc required
```

### 7.4 `StringPool` — The Interner Replacement

Architecture: **seeded xxHash3** for lookup, **BLAKE3** for `to_cid()` only,
**sharded `RwLock`** for multi-thread safety.

```
StringPool
├── shards: Box<[Shard; N]>           N from PoolConfig::shard_count (default 64)
│
└── Shard
    └── RwLock<ShardInner>
        ├── bytes: Vec<u8>            bump byte storage
        ├── slots: Vec<(u32, u32)>   (offset, len) per string
        ├── cids:  Vec<Option<[u8;16]>> lazy Cid cache per slot
        └── index: HashMap<u64, u32> xxHash3(seed, bytes) → slot index
```

```rust
impl StringPool {
    pub fn new(cfg: &PoolConfig) -> Self;

    /// Intern a string. Returns the existing Lid if already present.
    /// Lookup via xxHash3(seed, bytes); byte-confirms on hash hit.
    pub fn intern(&self, s: &str) -> Result<StrId, InternError>;

    /// Resolve a Lid to its string slice.
    pub fn get(&self, id: StrId) -> Option<&str>;

    /// Compute (or retrieve cached) BLAKE3 content address.
    /// Only call when cross-process stability is needed.
    pub fn to_cid(&self, id: StrId) -> Option<StrCid>;

    /// Look up without inserting.
    pub fn get_id(&self, s: &str) -> Option<StrId>;
}

#[derive(Debug)]
pub enum InternError {
    HashCollision { existing: StrId },
    CapacityExceeded,
}
```

**Bare-metal path:** `StaticStringPool<const CAP: usize>` — fixed-size
open-addressed table, `[u8; BYTES]` byte buffer, no `RwLock`, no heap.
`!Send + !Sync`, enforced at compile time. Used by `iot-min` preset.

### 7.5 `PathSegment for Lid<StrTag>` — Closing the Bridge

This is the impl that connects `dol-core`'s `PathSegment` trait to
`dol-cas`'s `StringPool`. It lives in `dol-cas` because `dol-core` must
not know about `dol-cas`.

```rust
// string_pool/segment_impl.rs

use dol_core::path::PathSegment;
use crate::{handle::tags::StrTag, handle::lid::Lid, string_pool::StringPool};

impl PathSegment for Lid<StrTag> {
    /// Resolution requires a `StringPool` reference.
    /// This prevents accidentally resolving interned paths without
    /// the pool needed to make the IDs meaningful.
    type Resolver = StringPool;

    #[inline]
    fn resolve<'a>(&'a self, resolver: &'a StringPool) -> &'a str {
        resolver.get(*self).unwrap_or("")
    }

    /// Returns the lazily-computed BLAKE3 content address for this string.
    /// Only computed on first call; cached in the pool thereafter.
    #[inline]
    fn content_id(&self, resolver: &StringPool) -> Option<[u8; 16]> {
        resolver.to_cid(*self).map(|cid| *cid.as_bytes())
    }
}
```

Now the full two-mode path bridge is closed:

```
Path<Name>         → resolver = &()          → no pool needed
Path<Lid<StrTag>>  → resolver = &StringPool  → pool-backed, compact
```

Both satisfy `Path<N: PathSegment>`. Lowering is the only place where
one becomes the other.

### 7.6 `ContentIndex` Stub

```rust
pub struct ContentIndex {
    node_hashes: HashMap<NodeId, [u8; 16]>,
}

impl ContentIndex {
    pub fn new() -> Self;
    pub fn get(&self, id: NodeId) -> Option<[u8; 16]>;
    pub fn invalidate(&mut self, id: NodeId);
}
```

The bottom-up walk that populates this is wired up in Phase 3 when
`ExprArena` exists.

### 7.7 Phase 2 Tests

- `StringPool::intern`: same string from two threads → same `StrId`
- `to_cid()` from two independent pools on the same string → same bytes
- `to_cid()` not computed until explicitly called (verify via slot inspection)
- `Lid<StrTag>::resolve` via `PathSegment` impl returns correct string
- `Lid<StrTag>::content_id` matches `pool.to_cid(id).unwrap().as_bytes()`
- `Path<Lid<StrTag>>` resolves target and fields correctly against a pool
- `StaticStringPool` compiles for `thumbv7em-none-eabihf`
- `Option<StrId>` is 4 bytes
- `size_of::<Cid<()>>() == 16`

---

## 8. Phase 3 — `dol-ir`

**Goal:** Complete expression IR (including the full `Expr<'a>` tree AST),
schema catalog, operations, program, and backend trait. Build a reference
no-op backend in parallel to validate the design throughout.

**Duration estimate:** 10–15 days. Largest phase.

**Critical:** Start a real backend (§11.2) no later than step 8.5. The
backend acts as a live integration test for the IR design. If translating an
`Operation` variant feels awkward, fix the IR — do not work around it.

### 8.1 `ExprNode` + Raw `ExprArena`

```rust
// expr/node.rs
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprNode {
    op:    u8,   // opcode family
    flags: u8,   // nullable, distinct, negated, …
    aux:   u16,  // secondary opcode within family
    a:     u32,  // operand A  (NodeId, LiteralId, FieldId, …)
    b:     u32,  // operand B
    c:     u32,  // operand C
}

// Asserted in tests AND xtask size-check — non-negotiable:
const _: () = assert!(core::mem::size_of::<ExprNode>() == 16);
```

**Typed constructors (raw field access never exposed outside `node.rs`):**

```rust
impl ExprNode {
    pub fn bin(op: BinOp, left: NodeId, right: NodeId) -> Self;
    pub fn unary(op: UnaryOp, operand: NodeId)         -> Self;
    pub fn lit_ref(id: LiteralId)                      -> Self;
    pub fn field_ref(id: FieldId)                      -> Self;
    pub fn func_ref(id: FuncId)                        -> Self;
    pub fn param(index: u16)                            -> Self;
    pub fn wildcard()                                   -> Self;
    pub fn count_all()                                  -> Self;
    // … one per opcode family
}
```

**Typed accessors:**

```rust
impl ExprNode {
    pub fn as_bin(&self)       -> Option<(BinOp, NodeId, NodeId)>;
    pub fn as_unary(&self)     -> Option<(UnaryOp, NodeId)>;
    pub fn as_lit_ref(&self)   -> Option<LiteralId>;
    pub fn as_field_ref(&self) -> Option<FieldId>;
    // …
}
```

**Opcode tables — append-only after v2 lock:**

```rust
// expr/ops.rs
#[repr(u16)]
#[non_exhaustive]
pub enum BinOp {
    Add = 0, Sub = 1, Mul = 2, Div = 3, Rem = 4,
    Eq  = 10, Ne = 11, Lt = 12, Le = 13, Gt = 14, Ge = 15,
    And = 20, Or = 21,
    BitAnd = 30, BitOr = 31, BitXor = 32, Shl = 33, Shr = 34,
    Concat = 40,
    Like = 50, ILike = 51,
    // Append new variants at the end only. Never renumber. Never reuse.
}

impl BinOp {
    pub fn try_from_u16(v: u16) -> Option<Self>;
}
```

### 8.2 Tree DSL — `Expr<'a>`

The complete tree AST as designed. The lifetime `'a` threads through
`Literal<'a>` for zero-copy string/byte borrows during construction.
Lowering erases it.

```rust
// expr/tree.rs

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum Expr<'a> {
    // ── References ──────────────────────────────────────────────────────────
    /// A structured path to a container, scoped container, or leaf attribute.
    ///
    /// - No field chain      → container / entity address
    /// - One or more fields  → leaf attribute or nested leaf
    /// - Namespace present   → authority / scope prefix on the above
    ///
    /// Lowers to `ExprNode::Ref` with an interned `Path<Lid<StrTag>>`.
    Ref(Path),

    // ── Values ───────────────────────────────────────────────────────────────
    /// A positional bind parameter: `$1`, `?`, `@p1`, `:1`, etc.
    /// DOL tracks only the presence of a slot; backends assign numbering.
    Param,

    /// A typed literal value from the unified `dol_core` type system.
    Lit(Literal<'a>),

    /// An ordered sequence of values: `[elem₀, elem₁, …]`.
    /// Backends render as array, tuple, multi-value constructor, or list.
    Seq(Vec<Expr<'a>>),

    /// A key-value mapping literal: `{ key: expr, … }`.
    /// Backends render as JSON object, map constructor, or document literal.
    Map(Vec<(Name, Expr<'a>)>),

    // ── Operations ───────────────────────────────────────────────────────────
    /// A binary operation: `left op right`.
    ///
    /// `OpDef` carries identity, precedence, associativity, and dialect hints.
    /// Negated binary forms (`NOT LIKE`, `NOT ILIKE`, …) use
    /// `Unary { op: UnaryOp::Not, … }`.
    Binary {
        left:  Box<Expr<'a>>,
        op:    OpDef,
        right: Box<Expr<'a>>,
    },

    /// A unary operation: `op expr`.
    /// Covers Not, Neg, BitNot, IsNull, IsNotNull.
    Unary {
        op:   UnaryOp,
        expr: Box<Expr<'a>>,
    },

    // ── Calls ────────────────────────────────────────────────────────────────
    /// A function or aggregate call: `func(args…)`.
    /// `FuncDef` carries identity, return type, and backend hints.
    /// `args` is empty for zero-argument functions such as `NOW()`.
    Call {
        func: FuncDef,
        args: Vec<Expr<'a>>,
    },

    // ── Structural ───────────────────────────────────────────────────────────
    /// A type coercion: `CAST(expr AS target_type)`.
    Cast {
        expr:        Box<Expr<'a>>,
        target_type: DataType,
    },

    /// A multi-arm conditional: first matching arm wins.
    /// `fallback: None` lets the backend substitute its native null value.
    /// Use `ConditionalBuilder` for ergonomic incremental construction.
    Match {
        arms:     Vec<(Expr<'a>, Expr<'a>)>,
        fallback: Option<Box<Expr<'a>>>,
    },

    /// A two-branch conditional: `IF cond THEN then_expr ELSE else_expr`.
    /// Distinct from a single-arm `Match`: both branches are known at
    /// construction time; backends may lower to ternary, IIF, or CASE.
    If {
        cond:      Box<Expr<'a>>,
        then_expr: Box<Expr<'a>>,
        else_expr: Box<Expr<'a>>,
    },

    /// Inclusive range containment: `low ≤ expr ≤ high`.
    /// For `NOT BETWEEN`, wrap with `Unary { op: UnaryOp::Not, … }`.
    InRange {
        expr: Box<Expr<'a>>,
        low:  Box<Expr<'a>>,
        high: Box<Expr<'a>>,
    },

    /// Set membership: `expr ∈ set`.
    /// For `NOT IN`, wrap with `Unary { op: UnaryOp::Not, … }`.
    /// Evaluation may short-circuit on the first match.
    MemberOf {
        expr: Box<Expr<'a>>,
        set:  Vec<Expr<'a>>,
    },

    // ── Projection decorators ─────────────────────────────────────────────────
    /// Attach an output label to an expression: `expr AS name`.
    /// Neutral replacement for SQL `AS alias`.
    Label {
        expr: Box<Expr<'a>>,
        name: Name,
    },

    /// All fields in the current projection scope.
    /// Backends expand to every visible field. Neutral replacement for `*`.
    Wildcard,

    /// Aggregate over the complete input with no field argument.
    /// Neutral replacement for `COUNT(*)`.
    /// Kept separate from `Call` so backends cannot apply DISTINCT,
    /// filter pushdown, or partial-aggregate rewrites to it.
    CountAll,

    // ── Scoping ──────────────────────────────────────────────────────────────
    /// Evaluate `expr` within an explicit dynamic scope.
    ///
    /// `Context` carries partitioning keys, ordering expressions, and an
    /// optional bounded frame. Backends translate this to their native
    /// construct: SQL `OVER (…)`, dataframe groupby+sort, IoT stream slice,
    /// time-series segment, pipeline window.
    ///
    /// Use `ContextBuilder` for fluent construction.
    Scoped {
        expr:    Box<Expr<'a>>,
        context: Context<'a>,
    },
}
```

**Rationale for key design decisions in this variant list:**

- `Ref(Path)` not `Field(FieldId)` — tree construction does not require schema
  resolution. The `Path` is resolved to a `FieldId` during lowering.
- `Scoped` over separate `WindowNode` side pools — `Context` is the general
  form for any evaluation scope (SQL window, IoT stream slice, time-series
  segment, pipeline partition). This reduces the need for separate stream/
  pipeline tree types.
- `Match` + `If` coexist — `If` with both branches at construction time is
  common enough to deserve its own variant; the optimizer and backends
  pattern-match on it without inspecting a single-arm `Match`.
- `CountAll` separate from `Call` — it carries no operand; backends must not
  apply `DISTINCT` or partial-aggregate rewrites to it.
- `MemberOf` + `InRange` with `Unary::Not` wrapping for negation — fewer
  variants, consistent rule, backends handle one negation pattern.

### 8.3 `Context<'a>` and Supporting Types

```rust
// expr/context.rs

pub struct Context<'a> {
    pub partition_by: Vec<Expr<'a>>,
    pub order_by:     Vec<OrderByExpr<'a>>,
    pub frame:        Option<Frame>,
}

pub struct Frame {
    pub unit:  FrameUnit,   // Rows | Range | Groups
    pub start: Boundary,
    pub end:   Boundary,
}

pub enum Boundary {
    Current,
    Before(Extent),
    After(Extent),
}

pub enum Extent {
    Unbounded,
    Offset(u64),
}

// expr/order.rs
pub struct OrderByExpr<'a> {
    pub expr:   Expr<'a>,
    pub dir:    SortDirection,
    pub nulls:  NullsOrder,
}

pub enum SortDirection { Asc, Desc }
pub enum NullsOrder    { First, Last, Default }

/// Fluent Context builder.
pub struct ContextBuilder<'a> {
    expr:         Expr<'a>,
    partition_by: Vec<Expr<'a>>,
    order_by:     Vec<OrderByExpr<'a>>,
    frame:        Option<Frame>,
}

impl<'a> ContextBuilder<'a> {
    pub fn new(expr: Expr<'a>) -> Self;
    pub fn partitioning(mut self, keys: impl IntoIterator<Item = Expr<'a>>) -> Self;
    pub fn ordering(mut self, exprs: impl IntoIterator<Item = OrderByExpr<'a>>) -> Self;
    pub fn between_positional(mut self, start: Boundary, end: Boundary) -> Self;
    pub fn build(self) -> Expr<'a>;  // returns Expr::Scoped
}

/// Fluent CASE builder.
pub struct ConditionalBuilder<'a> {
    arms:     Vec<(Expr<'a>, Expr<'a>)>,
    fallback: Option<Expr<'a>>,
}

impl<'a> ConditionalBuilder<'a> {
    pub fn new() -> Self;
    pub fn when(mut self, cond: Expr<'a>, result: Expr<'a>) -> Self;
    pub fn fallback(mut self, expr: Expr<'a>) -> Self;
    pub fn build(self) -> Expr<'a>;  // returns Expr::Match
}
```

### 8.4 Lowering — `Expr<'a>` → `ExprArena`

The lowering pass does three things simultaneously:
1. Converts the recursive `Expr<'a>` tree into the flat `ExprArena` representation
2. Converts `Path<Name>` segments to `Path<Lid<StrTag>>` by interning names into the `StringPool`
3. Threads `&mut Budget` at every recursive descent

```rust
// expr/lower.rs

pub fn lower(
    expr:    &Expr<'_>,
    arena:   &mut ExprArena,
    strings: &StringPool,
    budget:  &mut Budget,
) -> Result<NodeId, LowerError>;

/// Lower a Path<Name> to Path<Lid<StrTag>> by interning each segment.
/// Called inside `lower` when handling `Expr::Ref`.
pub fn lower_path(
    path:    &Path<Name>,
    strings: &StringPool,
    budget:  &mut Budget,
) -> Result<Path<Lid<StrTag>>, LowerError>;
```

The `lower_path` function is the single, authorised site where
`Path<Name>` becomes `Path<Lid<StrTag>>`. No other path in the codebase
should convert between these two forms.

**`LowerError`:**

```rust
#[derive(Debug)]
pub enum LowerError {
    BudgetExceeded(BudgetExceeded),
    InternFailed(InternError),
    ArenaOverflow,
}
```

### 8.5 `ContentIndex` Integration

```rust
// content_index/walk.rs (in dol-cas, wired up from dol-ir)

pub fn compute_hash(
    arena:  &ExprArena,
    id:     NodeId,
    index:  &mut ContentIndex,
    budget: &mut Budget,
) -> Result<[u8; 16], BudgetExceeded> {
    if let Some(h) = index.get(id) { return Ok(h); }
    budget.depth()?;
    budget.node()?;
    let node = arena.node(id);
    // hash = xxHash3_128(op_byte || child_hashes...)
    // insert into index; return hash
}
```

### 8.6 Schema Types

```rust
// schema/entity.rs
pub struct Entity {
    pub id:          EntityId,
    pub name:        StrId,           // interned in SchemaCatalog.strings
    pub fields:      Vec<FieldId>,
    pub constraints: Vec<Constraint>,
    pub relations:   Vec<RelationId>,
    pub policies:    Vec<PolicyId>,
}

// schema/field.rs
pub struct Field {
    pub id:       FieldId,
    pub name:     StrId,
    pub ty:       FieldType,
    pub nullable: bool,
    pub default:  Option<Literal<'static>>,
}

pub enum FieldType {
    Scalar(DataType),
    Relation(RelationId),
    Computed { expr: NodeId },
}

// schema/catalog.rs
pub struct SchemaCatalog {
    pub entities:  DynPool<Entity>,
    pub fields:    DynPool<Field>,
    pub relations: DynPool<Relation>,
    pub lookups:   DynPool<Lookup>,
    pub policies:  DynPool<Policy>,
    pub strings:   StringPool,     // all names interned here
}
```

### 8.7 `Operation` and `Program`

```rust
// command/operation.rs
#[non_exhaustive]
pub enum Operation {
    // Query
    Select(SelectOp),
    // Mutation
    Insert(InsertOp), Update(UpdateOp), Delete(DeleteOp), Upsert(UpsertOp),
    // DDL
    CreateEntity(CreateEntityOp), AlterEntity(AlterEntityOp), DropEntity(DropEntityOp),
    // ACL
    Grant(GrantOp), Revoke(RevokeOp),
    // Transaction
    Begin(BeginOp), Commit, Rollback,
    // Storage
    Compact(CompactOp), Checkpoint(CheckpointOp),
}

// command/program.rs
pub struct Program {
    pub id:         ProgramId,
    pub ops:        Vec<Operation>,
    pub schema_ref: Option<SchemaCid>,
    pub exprs:      ExprArena,
}
```

### 8.8 `Backend` Trait

```rust
// command/backend.rs
pub trait Backend {
    type Error: core::fmt::Debug;

    fn capabilities(&self) -> CapabilitySet;

    fn execute(
        &mut self,
        program: &Program,
        budget:  &mut Budget,
    ) -> Result<(), Self::Error>;

    fn execute_op(
        &mut self,
        op:     &Operation,
        exprs:  &ExprArena,
        budget: &mut Budget,
    ) -> Result<(), Self::Error>;
}

pub struct CapabilitySet { bits: u64 }

pub enum Capability {
    Transactions    = 1 << 0,
    Ddl             = 1 << 1,
    Acl             = 1 << 2,
    WindowFunctions = 1 << 3,
    Cte             = 1 << 4,
    FullTextSearch  = 1 << 5,
    JsonOperators   = 1 << 6,
    // extend at end only
}
```

### 8.9 Stream IR (feature = "stream")

```rust
pub struct StreamDef {
    pub id:        StreamId,
    pub source:    StrId,
    pub schema:    EntityId,
    pub watermark: WatermarkPolicy,
}

pub struct WindowSpec {
    pub kind:  WindowKind,  // Tumbling | Sliding | Session
    pub size:  Duration,
    pub slide: Option<Duration>,
}

pub enum WatermarkPolicy {
    EventTime { lag: Duration },
    ProcessingTime,
    Monotonic,
}
```

Note: `Expr::Scoped` covers most windowing semantics at the expression level.
`StreamDef` describes a data source, not an expression context.

### 8.10 Pipeline IR (feature = "pipeline")

```rust
pub struct PipelineGraph {
    pub nodes: DynPool<PipelineNode>,
    pub edges: Vec<Edge>,
}

pub enum PipelineNodeKind {
    Source(StreamId),
    Transform { expr: NodeId },
    Sink { target: StrId },
    Join  { left: PipelineNodeId, right: PipelineNodeId, on: NodeId },
    Aggregate { window: WindowSpec, group_by: Vec<FieldId>, agg: Vec<NodeId> },
}
```

### 8.11 Phase 3 Tests

- `ExprNode` is exactly 16 bytes (unit test + xtask size-check)
- Walking skeleton: `Expr::Binary { BinOp::Add, Lit(1), Lit(2) }` lowers correctly
- `lower_path` interns all segments; `Path<Lid<StrTag>>` resolves back to original strings
- `lower` respects budget depth: deeply nested `Expr` hits `BudgetExceeded::Depth`
- `intern_node` deduplicates structurally equal nodes
- `compute_hash` on the same subtree is deterministic across calls
- `Expr::Scoped` lowers to an arena node that references a `Context` side pool
- `Expr::MemberOf` with `Unary::Not` wrapper produces correct `NOT IN` semantics
- `BinOp::try_from_u16` rejects unknown opcodes
- Snapshot test: no existing `BinOp` numeric value changed (append-only guard)
- `SchemaCatalog` round-trips entity/field names through `StringPool`

---

## 9. Phase 4 — `dol-wire`

**Goal:** Tamper-evident envelope, budget-gated decode, round-trip
correctness, fuzz targets.

**Duration estimate:** 4–6 days.

### 9.1 `WireEnvelope`

```rust
pub struct WireEnvelope {
    pub version:    u16,
    pub content_id: [u8; 32],        // BLAKE3(payload) — tamper evidence
    pub schema_cid: Option<[u8; 16]>,
    pub payload:    Vec<u8>,
}

impl WireEnvelope {
    pub fn encode(program: &Program) -> Self {
        let payload    = postcard::to_allocvec(program).expect("encode");
        let content_id = dol_core::hash::content256(&payload);
        Self { version: WIRE_VERSION, content_id, schema_cid: None, payload }
    }

    pub fn verify(&self) -> Result<(), WireError> {
        let actual = dol_core::hash::content256(&self.payload);
        if actual != self.content_id { return Err(WireError::IntegrityFailure); }
        Ok(())
    }
}
```

### 9.2 `Decode` Trait

```rust
pub trait Decode: Sized {
    fn decode(bytes: &[u8], budget: &mut Budget) -> Result<Self, DecodeError>;
}

// Every impl must call budget.node() or budget.depth() before any recursive
// descent. xtask budget-gate enforces this.

// Decode for Lid<Tag>: validates NonZeroU32, rejects 0
// Decode for Operation: validates all FieldId / NodeId referents as in-bounds
// Decode for ExprArena: validates side-pool referents at every step
```

### 9.3 Round-Trip and Integrity Tests

```rust
#[test] fn program_round_trip() { /* encode → verify → decode → assert equal */ }
#[test] fn tampered_envelope_rejected() { /* flip one byte → WireError::IntegrityFailure */ }
#[test] fn budget_exceeded_on_deep_decode() { /* nested structure → BudgetExceeded::Depth */ }
#[test] fn null_lid_rejected() { /* 0u32 in payload → DecodeError::NullId */ }
#[test] fn oob_ref_rejected() { /* NodeId beyond arena len → DecodeError::InvalidRef */ }
```

### 9.4 Fuzz Targets

```rust
// fuzz/fuzz_targets/decode_program.rs
fuzz_target!(|data: &[u8]| {
    let mut budget = Budget::from_config(&BudgetConfig::default());
    let _ = Program::decode(data, &mut budget);
    // Must not panic, must not exceed budget without returning Err
});
```

---

## 10. Phase 5 — `dol-query`

**Goal:** Ergonomic query builder DSL. Quality of error messages is the
primary success metric — application developers see this layer more than any
other.

**Duration estimate:** 4–6 days.

### 10.1 `QueryContext`

```rust
pub struct QueryContext<'a> {
    pub schema:  &'a SchemaCatalog,
    pub strings: &'a StringPool,
    pub config:  &'a Config,
}
```

### 10.2 Query Builder Surface

Users of this API work entirely with `Path<Name>` and `Expr<'a>` with `Name`
segments. The `try_build` call is the only point where names are interned and
`Path<Name>` becomes `Path<Lid<StrTag>>`.

```rust
Query::from("orders")
    .get(["id", "total", "status"])
    .where_(|e| Expr::Binary {
        left:  Box::new(Expr::Ref(Path::new("orders").get("status"))),
        op:    OpDef::from(BinOp::Eq),
        right: Box::new(Expr::Lit(Literal::str("pending"))),
    })
    .order_by(OrderByExpr {
        expr:  Expr::Ref(Path::new("orders").get("created_at")),
        dir:   SortDirection::Desc,
        nulls: NullsOrder::Last,
    })
    .limit(100)
    .try_build(&ctx, &mut budget)?
```

### 10.3 `BuildError`

```rust
#[derive(Debug)]
pub enum BuildError {
    UnknownEntity { name: String },
    UnknownField  { entity: String, field: String, did_you_mean: Option<String> },
    TypeMismatch  { field: String, expected: DataType, got: DataType },
    BudgetExceeded(BudgetExceeded),
    LowerFailed(LowerError),
    SchemaViolation(String),
}
```

`UnknownField` includes `did_you_mean` computed via edit distance when
the candidate is unambiguous. Application developers should be able to act on
every error without reading source code.

---

## 11. Phase 6 — `dol` Umbrella + First Backend

**Goal:** Working end-to-end path from `Query::from(...)` to a real store.
The backend is the feedback mechanism for everything built in Phases 1–5.

**Duration estimate:** 5–8 days.

### 11.1 `dol` Umbrella

```rust
pub mod prelude {
    pub use dol_core::{DataType, Literal, Config, Budget, Name};
    pub use dol_core::path::{Path, PathSegment};
    pub use dol_cas::{Lid, Cid, Gid, StrId, StringPool};
    pub use dol_ir::expr::tree::Expr;
    pub use dol_ir::expr::op::{BinOp, UnaryOp, OpDef};
    pub use dol_ir::expr::context::{ContextBuilder, ConditionalBuilder};
    pub use dol_ir::{SchemaCatalog, Program, Operation, Backend};
    pub use dol_query::Query;
    pub use dol_wire::WireEnvelope;
}

pub struct Dol {
    pub config:  Config,
    pub schema:  SchemaCatalog,
    pub strings: StringPool,
}

impl Dol {
    pub fn builder() -> DolBuilder;
}
```

### 11.2 First Real Backend — In-Memory

Build `backends/memory/` first. It holds data in
`HashMap<EntityId, Vec<HashMap<FieldId, Literal<'static>>>>` and implements
every `Operation` variant.

Purpose:
1. **Validates `Operation` completeness** — if a common query is awkward to
   express, fix the IR before `dol-wire` and `dol-query` are built on top
2. **Provides a test fixture** for Phases 3–5 integration tests
3. **Documents the backend contract** for future SQL/Redis/REST backend authors

### 11.3 Second Backend — SQLite

`backends/sqlite/` follows immediately. SQLite is the best second choice
because it exercises every DDL and query path, its behaviour is
well-specified, and it surfaces real-world IR gaps the in-memory backend
cannot (joins, aggregates, subqueries, window functions, type coercions).

### 11.4 End-to-End Integration Test

```rust
#[test]
fn end_to_end_order_query() {
    let dol = Dol::builder().with_config(Config::standard()).build();

    dol.schema.define_entity("orders", |e| {
        e.field("id",         DataType::UInt64);
        e.field("status",     DataType::Text);
        e.field("total",      DataType::Decimal { precision: 10, scale: 2 });
        e.field("created_at", DataType::DateTime);
    });

    let ctx    = QueryContext::new(&dol.schema, &dol.strings, &dol.config);
    let mut budget = Budget::from_config(&dol.config.budget);

    let program = Query::from("orders")
        .get(["id", "total"])
        .where_(|_| Expr::Binary {
            left:  Box::new(Expr::Ref(Path::new("orders").get("status"))),
            op:    OpDef::from(BinOp::Eq),
            right: Box::new(Expr::Lit(Literal::str("pending"))),
        })
        .try_build(&ctx, &mut budget)
        .unwrap();

    let mut backend = MemoryBackend::new();
    backend.execute(&program, &mut budget).unwrap();
}
```

---

## 12. Cross-Cutting Concerns

### 12.1 The Two String Modes

This is the central string architecture. Every string-bearing API falls into
exactly one of these two modes:

| Mode | Type | Storage | Cost | Resolver | Used in |
|---|---|---|---|---|---|
| Inline | `Name` | `&'static str` or `Box<str>` | 16–24 B | `()` | Tree DSL, query builder, diagnostics, opcode labels |
| Interned | `Lid<StrTag>` = `StrId` | 4 B handle, bytes in `StringPool` | 4 B | `&StringPool` | ExprArena, SchemaCatalog, post-lowering IR |

`PathSegment` is the trait that lets `Path<N>` work with either.
`lower_path` is the single authorised conversion site.

No other mechanism converts between modes. No implicit interning on
construction. No `Name` in a `DynPool`. No `String` inside `ExprNode`.

### 12.2 `no_std` Strategy

| Crate | `no_std` | `alloc` | static-only |
|---|---|---|---|
| `dol-core` | ✅ | optional | ✅ (`Path` uses inline array) |
| `dol-cas` | ✅ | optional | ✅ (`StaticPool`, `StaticStringPool`) |
| `dol-ir` | ✅ | required | ❌ |
| `dol-wire` | ✅ | required | ❌ |
| `dol-query` | ✅ | required | ❌ |
| `dol` (iot-min) | ✅ | optional | ✅ |

Every crate starts with `#![no_std]` at the top of `lib.rs`.
`std`-specific code lives behind `#[cfg(feature = "std")]`.

### 12.3 `Path` Field-Chain Storage — IoT Note

Under `feature = "alloc"`: `SmallVec<[N; 2]>` — inline for 0–2 segments,
heap-allocated beyond that. This covers the overwhelming majority of paths.

Under bare-metal (`no alloc`): `[Option<N>; 4]` with a `u8` length counter.
Paths with more than 4 field segments return `PathError::TooDeep` at
construction time. Four segments cover virtually all IoT schema paths
(`device.sensors.temp.value`). The constant `4` is a compile-time tunable
via const generic if needed.

### 12.4 Serde Policy

| Type | Serialize | Deserialize |
|---|---|---|
| `Lid<Tag>` | ✅ as `u32` | ❌ never |
| `Cid<Tag>` | ✅ as `[u8;16]` | ❌ never |
| `Gid<Tag>` | ✅ as `[u8;32]` | ❌ never |
| `Name` | ✅ as `str` | ❌ never |
| `Path<Name>` | ✅ | ❌ never |
| `ExprNode` | ✅ | ❌ never |
| `Operation` | ✅ | ❌ never |
| `SchemaCatalog` | ✅ | ❌ never |
| `WireEnvelope` | ✅ | ✅ sole permitted Deserialize site |

`WireEnvelope` is the single permitted `Deserialize` site. Everything else
goes through `dol_wire::Decode`.

### 12.5 Budget Gate Enforcement

`xtask budget-gate` scans every public function signature in `lib/` and
asserts that any function whose name begins with `walk_`, `visit_`,
`lower_`, `decode_`, or `content_hash_` accepts `&mut Budget`. CI fails
if this check fails.

`lower_path` is additionally budget-checked — it iterates segments and each
call to `strings.intern()` counts against `budget.node()`.

### 12.6 Panic Policy

Permitted panics (each carries `// design-time error: <reason>` and has a
fallible sibling):

- `DynPool::push` when `len == u32::MAX` (4 billion entries ≈ tens of GiB)
- `ExprNode` typed constructors when opcode encoding overflows (caught in
  unit tests before ever reaching production)

All other panics are bugs.

### 12.7 Version and Stability Tiers

| Tier | Crates | Policy |
|---|---|---|
| A (locked) | `dol-core`, `dol-cas` | No breaking changes after v1.0. Additive-only. |
| B (stable) | `dol-wire` | Wire format versioned separately. Breaking changes require new `version` field. |
| C (evolving) | `dol-ir`, `dol-query` | Semver; breaking changes with major bump. |
| D (experimental) | `dol` presets, backends | Unstable until explicitly promoted. |

`BinOp`, `UnaryOp`, and all opcode tables are append-only after v2 lock.
Never renumber. Never reuse. Enforced by snapshot tests.

---

## 13. Testing Strategy

### 13.1 Unit Tests (per module)

Co-located `#[cfg(test)] mod tests` block in every module. No test may
depend on another crate's internals.

### 13.2 Integration Tests (per crate)

`tests/` directory per crate, using only the crate's public API.

### 13.3 Cross-Crate Integration Tests

`dol/tests/integration/` — full path from query builder through backend
execution. These are the first tests to break when a cross-crate interface
drifts.

### 13.4 Property Tests (`proptest`)

| Target | Property |
|---|---|
| `StringPool::intern` | Idempotent: same string always returns same `StrId` |
| `Cid` | Same bytes always produce same `Cid` regardless of pool state |
| `lower_path` | `lower_path(p, pool)` followed by `resolve(pool)` reconstructs original strings |
| `Path<Name>` equality | `Path::new("a").get("b") == Path::new("a").get("b")` |
| `ExprArena` | `lower(expr, arena, strings)` followed by `compute_hash` is deterministic |
| `WireEnvelope` | `encode → verify → decode` is a no-op |
| `BudgetExceeded` | Deeply nested `Expr` always triggers `BudgetExceeded::Depth` |
| `BinOp` | `try_from_u16(op.as_u16()) == Some(op)` for all known variants |

### 13.5 Fuzz Targets (nightly CI)

```
fuzz/fuzz_targets/
  decode_program.rs    arbitrary bytes → Program::decode
  decode_schema.rs     arbitrary bytes → SchemaCatalog::decode
  decode_expr.rs       arbitrary bytes → ExprArena decode
  intern_strings.rs    arbitrary UTF-8 → StringPool::intern
  lower_expr.rs        arbitrary Expr tree → lower()
```

### 13.6 MIRI

`dol-core` and `dol-cas` run under MIRI on every CI push.
`dol-ir` joins MIRI once `ExprArena` stabilises.

### 13.7 Snapshot Tests (`insta`)

- `BinOp`, `UnaryOp` numeric values — catch accidental renumbering
- `WireEnvelope` for a canonical test program — catch wire format drift
- `SchemaCatalog` serialization for a canonical schema — catch schema IR drift
- `Path<Name>` serialization — catch path format drift

---

## 14. CI Pipeline

```
┌─────────────────────────────────────────────────────┐
│  Push / PR                                          │
│                                                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │  format  │  │   lint   │  │   test matrix    │  │
│  │ cargo fmt│  │ clippy   │  │ stable + MSRV    │  │
│  │ --check  │  │ pedantic │  │ Linux/Mac/Win    │  │
│  └──────────┘  └──────────┘  └──────────────────┘  │
│                                                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │ no-std   │  │  xtask   │  │      MIRI        │  │
│  │ ARM bare │  │ budget-  │  │  core + cas      │  │
│  │  metal   │  │ gate +   │  │  (nightly)       │  │
│  │          │  │ size +   │  │                  │  │
│  │          │  │ dag      │  │                  │  │
│  └──────────┘  └──────────┘  └──────────────────┘  │
│                                                     │
│  ┌──────────────────────────────────────────────┐   │
│  │  fuzz  (short on PR; long on merge to main)  │   │
│  └──────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

**Blocking gates** (PR cannot merge if any fail):
format, lint, test matrix, no-std, xtask checks.

**Non-blocking on PR** (block on nightly main):
MIRI, fuzz, size-report regression.

---

## 15. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `Operation` enum incomplete for real backends | High | High | Build SQLite backend during Phase 3, not after. Every awkward translation is an IR fix signal. |
| `ExprNode` pressure to exceed 16 bytes | Medium | High | Side-pool discipline: any new opcode needing >3 u32 operands gets a side pool. CI size-check fails immediately. |
| `lower_path` hot-path overhead | Medium | Medium | Profile before optimising. `StringPool::intern` is O(1) amortised. If `Path<Name>` segments are repeat-heavy, a local `HashMap<&str, StrId>` cache in the lowering pass eliminates redundant hashing. |
| `SmallVec` dep in `dol-core` | Low | Low | Gated on `feature = "alloc"`. Bare-metal path uses inline array. If `SmallVec` causes issues, replace with a hand-rolled two-inline variant. |
| `PathSegment::content_id` called eagerly in hot loops | Low | Medium | Document it as "call only when cross-process stability is needed". Backends that do plan caching call it once per compilation, not per node traversal. |
| `StringPool` shard count wrong for workload | Medium | Low | `PoolConfig::shard_count` is runtime-configurable. Default 64 covers most workloads. |
| Wire format churn during Phase 3 IR changes | High | Medium | Do not stabilise wire format until `dol-ir` is stable. `WireEnvelope::version` field handles this. Snapshot tests catch accidental drift. |
| `Backend` trait wrong granularity | Medium | High | In-memory + SQLite backend provide early feedback. Trait is `#[non_exhaustive]` until v1.0. |
| `no_std` breakage from dependency updates | Low | Medium | `xtask no-std-check` runs on every push against `thumbv7em-none-eabihf`. |
| Opcode accidental renumbering | Low | High | Snapshot tests on numeric values. `#[non_exhaustive]` prevents external exhaustive matching. |
| Budget gate missed on new function | Medium | Low | `xtask budget-gate` is CI-blocking and mechanical. |

---

## 16. Milestone Summary

| Milestone | Deliverable | Gate |
|---|---|---|
| M0 | Workspace compiles, CI green on empty crates | All xtask stubs pass |
| M1 | `dol-core` complete: config, budget, hash, `Name`, `PathSegment`, `Path<N>`, types, span, diagnostic | MIRI clean; ARM cross-compile clean; `Path<Name>` unit tests pass |
| M2 | `dol-cas` complete: handles, pools, `StringPool`, `PathSegment for Lid<StrTag>` | Concurrent intern test; `StaticStringPool` on ARM; two-mode path round-trip test |
| M3a | `ExprNode` + `ExprArena` + opcode tables | `ExprNode == 16B`; walking skeleton test |
| M3b | Full `Expr<'a>` tree DSL + lowering incl. `lower_path` | All `Expr` variants lower correctly; budget depth test |
| M3c | Schema catalog + `Operation` + `Backend` trait | In-memory backend executes `SelectOp` |
| M3d | SQLite backend: CRUD | Full CRUD via `dol-ir` against SQLite; IR gaps resolved |
| M3e | `ContentIndex` integrated; Merkle walk | Deterministic hash for identical programs |
| M4 | `dol-wire` complete; fuzz targets green | Round-trip test; tamper detection; budget decode test |
| M5 | `dol-query` complete | End-to-end test from DSL to SQLite execution |
| M6 | `dol` umbrella; `iot-min` preset | IoT demo binary compiles for ARM; size under budget |
| V1-LOCK | Wire format snapshot committed; opcode table snapshot committed | No snapshot diff on main |

---

*This document is the single source of truth for the rewrite.
Update it when architectural decisions change.
Do not let the code and this document diverge.*
