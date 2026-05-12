# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### v2 rewrite — Phase 3c-β: `dol-ir` frame primitives (`dol-rewrite-plan-v2.md` §8.3 — `Expr<'a>`-independent slice)

**Second M3c slice. Adds the parts of §8.3 that carry no `Expr<'a>`
operands — pure POD value types describing window/scope frames and
ordering — so they can be reviewed in isolation ahead of the tree
DSL itself (M3c-γ).**

- **New module `dol_ir::expr::frame`** (`lib/dol-ir/src/expr/frame.rs`):
  - `FrameUnit` — `#[non_exhaustive]` Rows / Range / Groups.
  - `Extent` — Unbounded / Offset(u64).
  - `Boundary` — Current / Before(Extent) / After(Extent), with
    `unbounded_preceding` / `unbounded_following` / `preceding(n)` /
    `following(n)` const helpers.
  - `Frame { unit, start, end }` with `rows` / `range` / `groups`
    const constructors. Public fields by design — pure POD.
  - All types are `Copy + Eq + Hash`; no `'a` lifetime, no
    allocation, no dependence on `Expr<'a>`.

- **New module `dol_ir::expr::order`** (`lib/dol-ir/src/expr/order.rs`):
  - `SortDirection` — Asc (default) / Desc, with `is_ascending` and
    `reversed` helpers (involutive).
  - `NullsOrder` — First / Last / Default (default — defers to the
    backend's native rule).
  - Both `Copy + Eq + Hash + Default`.

- **v2 conventions honoured**:
  - Growable enums (`FrameUnit`) are `#[non_exhaustive]`. `Boundary`
    / `Extent` / `SortDirection` / `NullsOrder` are intentionally
    closed — their variant set is dictated by the SQL-92 frame
    grammar and would not grow without a wire-format change.
  - All public fns are `#[must_use]` where appropriate; constructors
    are `const fn`.
  - No serde — `dol-ir` has no serde feature yet.

- **12 new tests** (6 in `frame`, 6 in `order`); `dol-ir` lib **62 → 74 passing**, workspace **213 → 225 passing**. Coverage: variant equality and inequality
  across all `Boundary` / `Extent` / `FrameUnit` / `SortDirection` /
  `NullsOrder` shapes; `Boundary` helpers match explicit
  construction; `Frame::rows`/`range`/`groups` set the right unit;
  `Boundary::Current` is distinct from `preceding(0)` / `following(0)`;
  every type is `Copy + Eq + Hash`; `SortDirection::default() ==
  Asc`, `NullsOrder::default() == Default`; `reversed()` is
  involutive.

- **All acceptance gates green**: build, `cargo test --workspace
  --all-features` (225 passing), `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`, `cargo fmt --check`,
  `cargo doc -D warnings -D rustdoc::broken_intra_doc_links`,
  `cargo check -p dol-ir --target thumbv7em-none-eabihf
  --no-default-features` (no_std + alloc), all five `cargo xtask`
  subcommands.

### Out of scope for M3c-β (future M3c-γ … M3c-δ + M3d/M3e)

- `Expr<'a>` enum (§8.2) with all 14 variants.
- `Context<'a>` and `OrderByExpr<'a>` (§8.3) — both carry `Expr<'a>`
  operands, so they ride with the tree DSL.
- `ContextBuilder` / `ConditionalBuilder` (fluent builders for
  `Expr::Scoped` and `Expr::Match`).
- Well-known function registry (`LENGTH` / `UPPER` / `COUNT` /
  `SUM` / …) plus `DolFunc` / `DolOp` trait scaffolding and
  `define_func!` / `define_op!` macros.
- Lowering pipeline `Expr<'a>` → `ExprArena` and `lower_path` (§8.4
  main body) — also the natural home for `impl PathSegment for
  Lid<StrTag>` deferred from M2.
- Schema (§8.6), `Operation` / `Program` (§8.7), `Backend` (§8.8),
  stream/pipeline (§8.9–§8.10).

### v2 rewrite — Phase 3c-α: `dol-ir` tree-DSL leaf metadata (`dol-rewrite-plan-v2.md` §8.2 — first slice)

**First M3c slice. Adds the leaf metadata types that the forthcoming
`Expr<'a>` enum and lowering pipeline will both depend on, with no
`Expr<'a>` itself yet so the metadata can be reviewed in isolation.**

- **New module `dol_ir::expr::meta`** (`lib/dol-ir/src/expr/meta.rs`):
  - `OpCategory` — 7-variant `#[non_exhaustive]` enum classifying
    binary operators (Comparison / NullSafe / Arithmetic / Logical /
    Pattern / StringOp / Bitwise). One-to-one with the high-level
    bands of the wire-side `BinOp` (M3a).
  - `OpDef { name: Name, kind: OpCategory }` — tree-side operator
    carrier with `new_static` (zero-alloc) / `custom` constructors,
    `Display` and `PartialEq<&str>` impls. Stores names as
    `dol_core::strings::Name` (Static `&'static str` or Owned
    `Box<str>`).
  - **Well-known operator constants**: 21 `pub const &'static str`
    names spanning Comparison (EQ/NE/LT/GT/LE/GE), Arithmetic
    (ADD/SUB/MUL/DIV/MOD), Logical (AND/OR), Bitwise (BIT_AND/
    BIT_OR/BIT_XOR/SHIFT_LEFT/SHIFT_RIGHT), String (CONCAT), Pattern
    (LIKE/ILIKE).
  - `OpDef::well_known() -> &'static [(&'static str, OpCategory)]` —
    tabulated catalogue, **wire-format-stable**. A snapshot test
    locks both length (21) and per-band boundary entries; adding /
    removing / reordering an entry is a wire-format change, not a
    casual edit.
  - `Arity { Exact(u8), AtLeast(u8), Range(u8, u8), Any }` and
    `ArityError { func_name, expected, actual }` with a `Display`
    that names the function and reports the constraint shape.
  - `FuncKind` — `#[non_exhaustive]` Scalar / Aggregate / Window.
  - `FuncDef { name: Name, arity: Arity, kind: FuncKind }` with
    `new_static` / `custom` (Scalar + Any defaults) / `custom_with`,
    plus `validate_arity(arg_count) -> Result<(), ArityError>` that
    correctly handles all four `Arity` shapes.

- **v2 conventions honoured**:
  - `Serialize`-only — no `Deserialize` derive in this slice (and
    `dol-ir` has no serde feature yet; one will be added when wire
    support arrives in a later phase).
  - All new public enums that may grow are `#[non_exhaustive]`.
  - All public fns are `#[must_use]` where appropriate; constructors
    are `const fn` where they take only `&'static str` / primitive
    inputs.

- **15 new tests**, workspace **198 → 213 passing**. Coverage:
  - `OpDef`: static / custom round-trip; equality across storage
    variants; inequality across name and category fields; `Display`
    matches `name()`; symmetric `PartialEq<&str>`; well-known
    catalogue locked at 21 entries with explicit boundary checks;
    well-known names are unique; hash is value-equal across Static /
    Owned representations.
  - `FuncDef`: static const construction; `custom` defaults; explicit
    arity + kind override; inequality across all three fields.
  - `validate_arity`: every `Arity` shape (Exact / AtLeast / Range /
    Any) including the boundary cases (`Range(1,2)` rejects 0 and 3,
    `Any` accepts 0..=255).
  - `ArityError::Display` includes the function name, the constraint
    shape ("exactly 1" / "at least 2" / "1..=2"), and the actual
    count.
  - `FuncKind` variants are distinct under `Debug`.

- **All acceptance gates green**: build, `cargo test --workspace
  --all-features` (213 passing), `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`, `cargo fmt --check`,
  `cargo doc -D warnings -D rustdoc::broken_intra_doc_links`,
  `cargo check -p dol-ir --target thumbv7em-none-eabihf
  --no-default-features` (no_std + alloc), all five `cargo xtask`
  subcommands.

### Out of scope for M3c-α (future M3c-β … M3c-δ + M3d/M3e)

- `Expr<'a>` enum (§8.2) with all 14 variants and the `Context<'a>` /
  `Frame` / `Boundary` / `Extent` / `OrderByExpr` / `SortDirection` /
  `NullsOrder` family (§8.3) — including `ContextBuilder` and
  `ConditionalBuilder`.
- Well-known function registry (`LENGTH` / `UPPER` / `LOWER` /
  `COUNT` / `SUM` / `AVG` / `ROW_NUMBER` / …) plus the `DolFunc` /
  `DolOp` trait scaffolding and `define_func!` / `define_op!`
  macros (legacy carried these as zero-sized marker types — porting
  is a bigger surface that earns its own PR).
- Lowering pipeline `Expr<'a>` → `ExprArena` and `lower_path` (§8.4
  main body) — also the natural home for `impl PathSegment for
  Lid<StrTag>` deferred from M2.
- Schema types `Entity` / `Field` / `SchemaCatalog` (§8.6).
- `Operation` / `Program` (§8.7).
- `Backend` trait + reference no-op backend (§8.8).
- Optional `stream` / `pipeline` features (§8.9–§8.10).

### v2 rewrite — Phase 3b: `dol-ir` arena dedup + content-hash walker (`dol-rewrite-plan-v2.md` §8.4 dedup, §8.5)

**Two pure additions to the `ExprArena` surface shipped in M3a. Both
unblock the tree-DSL lowering that lands in M3c.**

- **`ExprArena::intern_node`** (`lib/dol-ir/src/expr/arena.rs`) —
  structural-dedup constructor. Returns the existing `NodeId` for a
  byte-identical node, otherwise pushes and records the new id.
  - Index keyed by `dol_core::hash::fast64` (xxHash3, in-process) over
    the 16 raw bytes of `ExprNode`. Collisions are resolved by
    comparing the actual node bytes after a hash hit; on a true
    collision the newcomer wins a fresh slot (panic-free).
  - Lazy `HashMap<u64, NodeId>` built on first call.
  - Raw `push` is unchanged and intentionally bypasses the index;
    the two modes interoperate without surprises (documented).
  - `dedup_len()` test/diagnostic helper exposes the index size.

- **`expr::walk::content_hash`** (`lib/dol-ir/src/expr/walk.rs`) —
  bottom-up walker that derives the BLAKE3-128 content address of any
  subtree, memoised in `dol_cas::content_index::ContentIndex`.
  - Hash domain: `[0x01, family, flags, aux_lo, aux_hi, ...]` then
    either child content addresses (recursive families) or the raw
    12-byte operand slab (leaves). The leading `0x01` is the wire
    domain tag — bumping it invalidates every persisted digest.
  - Threads `&mut Budget` per descent (`budget.depth()` + `budget.node()`).
    Cache hits skip the budget entirely — they are free lookups.
  - Returns `ContentHashError { Budget(BudgetExceeded), MissingNode,
    UnknownOpcode }`.
  - Cross-process stable: two arenas building the byte-identical
    subtree (with completely different internal `NodeId`s) derive the
    same digest. Test `structural_equality_yields_identical_hash`
    locks this guarantee.

- **`expr::walk::content_hash_bytes`** — convenience wrapper around the
  project's `content128` chokepoint for callers that want auxiliary
  digests in the same hash domain. Carries an explicit
  `// budget-gate: opt-out` marker (one-shot byte hash, no recursion).

- **`Cargo.toml`** — `dol-ir` now depends on `hashbrown` (workspace
  default-features off) for the dedup map. `dol-core/hash` was already
  pulled in M3a.

- **`dol_cas::ContentIndex`** doc updated to clarify the BLAKE3-128
  cross-process stability of stored digests and to point at the new
  walker (without creating a cross-crate intra-doc link cycle).

- **17 new tests**, workspace 181 → **198 passing**. Coverage:
  - `intern_node` round-trip; distinct nodes get distinct ids;
    structural-subtree sharing through `BinOp` parents; flag
    differences create distinct dedup keys; raw `push` does not
    populate the dedup index (mode-mixing).
  - `content_hash` determinism, cache-hit avoids budget cost, distinct
    leaves / opcodes / operand orders / flags all produce distinct
    digests, structural equality across arenas yields identical digest,
    `MissingNode` on dangling children, depth + node budget exhaustion
    each surface the right `BudgetExceeded` variant, `content_hash_bytes`
    matches `content128`.

- **All acceptance gates green**: build, `cargo test --workspace
  --all-features`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, `cargo fmt --check`, `cargo doc -D
  warnings -D rustdoc::broken_intra_doc_links`, `cargo check -p dol-ir
  --target thumbv7em-none-eabihf --no-default-features`, all five
  `cargo xtask` subcommands exit 0 (including `budget-gate` after the
  opt-out marker on `content_hash_bytes`).

### Out of scope for M3b (covered in M3c–M3e)

- Tree DSL `Expr<'a>` (§8.2), `Context<'a>` / `Frame` / `OrderByExpr`
  (§8.3), `ContextBuilder` / `ConditionalBuilder`.
- Full lowering pipeline `Expr<'a>` → `ExprArena` and `lower_path`
  (`Path<Name>` → `Path<Lid<StrTag>>`) per §8.4 main body. This is
  also the natural home for the deferred `impl PathSegment for
  Lid<StrTag>` from M2.
- Schema types `Entity` / `Field` / `SchemaCatalog` (§8.6).
- `Operation` / `Program` (§8.7).
- `Backend` trait + reference no-op backend (§8.8).
- Optional `stream` / `pipeline` features (§8.9–§8.10).

### v2 rewrite — Phase 3a: `dol-ir` foundation (`dol-rewrite-plan-v2.md` §8.1)

**M3 is the largest milestone in the rewrite (10–15 days, plan §8).
To keep PRs reviewable it is split across M3a–M3e. PR #4 ships the
foundation everything else in M3 builds on.**

- **`ExprNode`** (`lib/dol-ir/src/expr/node.rs`) — the 16-byte v2
  expression record. `#[repr(C)]`, `bytemuck::Pod + Zeroable`. Fields
  `op:u8 + flags:u8 + aux:u16 + a:u32 + b:u32 + c:u32`.
  - **16-byte invariant** asserted three ways: `const _: () = assert!(...)`
    in the source, runtime test in `node::tests`, and `xtask
    size-check` (now wired up — see below).
  - Raw `a` / `b` / `c` fields are private. Construction goes through
    typed constructors (`bin`, `unary`, `lit_ref`, `field_ref`,
    `func_ref`, `param`, `wildcard`, `count_all`); reading goes
    through typed accessors (`as_bin`, `as_unary`, `as_lit_ref`,
    `as_field_ref`, `as_func_ref`, `as_param`, `is_wildcard`,
    `is_count_all`). Each constructor↔accessor pair has a round-trip
    test.

- **Opcode tables** (`lib/dol-ir/src/expr/ops.rs`) per plan §8.1:
  - `OpFamily` (top-level discriminator, `#[repr(u8)] #[non_exhaustive]`)
    with values `Reserved=0`, `Bin=1`, `Unary=2`, `LitRef=3`,
    `FieldRef=4`, `FuncRef=5`, `Param=6`, `Wildcard=7`, `CountAll=8`.
  - `BinOp` (`#[repr(u16)] #[non_exhaustive]`) — arithmetic 0–4,
    comparison 10–15, logical 20–21, bitwise 30–34, concat 40,
    pattern-match 50–51. Banded so future opcodes can be appended
    inside each group without disturbing existing numbers.
  - `UnaryOp` (`#[repr(u16)] #[non_exhaustive]`) — `Not=0`, `Neg=1`,
    `BitNot=2`, `IsNull=10`, `IsNotNull=11`.
  - All three have `try_from_*` decoders that return `None` for
    unknown values (the wire-format / version-skew error).
  - **Numeric-stability snapshot tests** lock every existing value;
    accidentally renumbering an opcode fails CI.

- **`NodeFlags`** (`lib/dol-ir/src/expr/flags.rs`) — `#[repr(transparent)]
  u8` bitset with const builders `with_nullable` / `with_distinct` /
  `with_negated` / `with_aggregate` and matching predicates. Custom
  `Debug` lists active flags by name. Round-trips through
  `from_bits` / `to_bits` preserving unused bits.

- **`ExprArena`** (`lib/dol-ir/src/expr/arena.rs`, `feature = "std"`)
  — flat `DynPool<ExprNode>`-backed store keyed by `NodeId`. Raw
  `push(node) -> Option<NodeId>` / `get(id) -> Option<&ExprNode>` /
  `len` / `is_empty` / `with_capacity`. Ids are dense and one-based
  (matches the `Lid` contract). **Dedup index lands in M3b** with
  lowering — no caller in M3a can produce duplicates worth deduping.

- **xtask `size-check`** (`xtask/src/main.rs`) — replaces the M0 stub
  with real CI-grade assertions: `size_of::<ExprNode>() == 16` and
  `size_of::<Option<NodeId>>() == size_of::<Option<StrId>>() == 4`.
  `xtask` now depends on `dol-ir` and `dol-cas` so the assertions
  run against the canonical types.

- **Cargo.toml** — `dol-ir` depends on `dol-core` (`hash` feature),
  `dol-cas`, and `bytemuck`. `default = ["std"]`. No-std build of
  `dol-ir --no-default-features` compiles cleanly on
  `thumbv7em-none-eabihf`; only the std-gated `ExprArena` is
  unavailable in that mode.

- **30 new tests** (workspace 151 → **181 passing**). Covers:
  16-byte / 4-byte alignment invariants, every opcode round-trip,
  unknown-opcode rejection, numeric-stability snapshot for
  `OpFamily` / `BinOp` / `UnaryOp`, `NodeFlags` independence and
  unused-bit preservation, every `ExprNode::*` constructor /
  accessor pair, `Pod` byte-cast round-trip, and `ExprArena` push /
  get / dense-id / nested-reference scenarios.

### Out of scope for M3a (covered in M3b–M3e)

- Tree DSL `Expr<'a>` (§8.2), `Context<'a>` / `Frame` / `OrderByExpr`
  (§8.3), `ContextBuilder` / `ConditionalBuilder`.
- Lowering `Expr<'a>` → `ExprArena` and `lower_path`
  (`Path<Name>` → `Path<Lid<StrTag>>`) per §8.4. This is also the
  natural home for the deferred `impl PathSegment for Lid<StrTag>`
  from M2.
- `ContentIndex` walker `compute_hash` (§8.5).
- Schema types `Entity`, `Field`, `SchemaCatalog` (§8.6).
- `Operation`, `Program` (§8.7).
- `Backend` trait, `CapabilitySet` / `Capability`, reference no-op
  backend (§8.8).
- Optional `stream` / `pipeline` features (§8.9–§8.10).

### v2 rewrite — Phase 2: `dol-cas` (`dol-rewrite-plan-v2.md` §7)

**M2 ships the identity and pooling layer that sits between
`dol-core` and the upcoming `dol-ir` (M3): handle types, generic
arenas, the `StringPool` that replaces v1's `dol_expr::Interner`, and
a `ContentIndex` stub.**

- **Handle types** (`lib/dol-cas/src/handle/`) per plan §7.1–§7.2.
  - `Lid<Tag>` is a re-export of `dol_core::ids::Id` — the planned
    struct is byte-identical to the existing primitive; aliasing
    avoids two copies of the same niche-optimised handle. `Option<Lid<Tag>>`
    is 4 bytes.
  - `Cid<Tag>` (`#[repr(C)] { raw: [u8; 16], _: PhantomData<fn() -> Tag> }`):
    BLAKE3-128 cross-process stable content address. Hand-written
    trait impls without a `Tag` bound. `Serialize` only (no
    `Deserialize`; wire-in goes through `dol-wire::Decode`).
  - `Gid<Tag>` (same shape with `[u8; 32]`): full BLAKE3-256 for
    signed manifests and tamper-evident wire envelopes.
  - Tag set: `StrTag`, `NodeTag`, `FieldTag`, `EntityTag`,
    `LiteralTag`, `FuncTag`, `SchemaTag`, `ProgramTag`.
  - Aliases: `StrId`, `NodeId`, `FieldId`, `EntityId`, `LiteralId`,
    `FuncId`, `StrCid`, `SchemaCid`, `ProgramCid`, `ProgramGid`.

- **Generic arenas** (`lib/dol-cas/src/pool/`) per plan §7.3.
  - `ArenaStorage<T>` trait with `push`/`get`/`get_mut`/`len`.
  - `DynPool<T>` (Vec-backed, std-only) and `StaticPool<T, CAP>`
    (no-alloc, `[Option<T>; CAP]`-backed).
  - Both expose `push_id<Tag>() -> Option<Lid<Tag>>` /
    `get_by_id<Tag>(Lid<Tag>) -> Option<&T>` for typed access.

- **`StringPool`** (`lib/dol-cas/src/string_pool/dynamic.rs`,
  `feature = "std"`) per plan §7.4.
  - Single-locked `RwLock<Inner>` interner (sharding deferred — see
    note below). xxHash3 (`fast64_seeded`) for lookup with byte-confirm
    on hash hit; lazy BLAKE3-128 (`content128`) for `to_cid()`.
  - `intern(s) -> Result<StrId, InternError>`. `InternError` variants:
    `HashCollision { existing }`, `CapacityExceeded`, `Poisoned`.
  - `to_cid(id)` is lazy: first call computes and caches in the slot's
    `Option<[u8; 16]>` cell; subsequent calls return the cached value.
  - `get_id(s)` and `get(id)` for read-only lookup. (Note: `get`
    returns owned `String` because bytes live behind `RwLock`; the
    sharded rewrite in M3 will provide zero-copy borrowed access.)
  - Cross-pool stability: two independent `StringPool`s produce the
    same `Cid` for the same input.

- **`StaticStringPool<BYTES, SLOTS>`** (`lib/dol-cas/src/string_pool/static.rs`).
  - Fixed-capacity, single-threaded, `const fn new(seed)` so it can
    live in `static` storage.
  - `!Send + !Sync` via a `PhantomData<*const ()>` field.
  - `StaticInternError { SlotsExhausted, BytesExhausted, HashCollision { existing } }`.
  - Compiles cleanly on `thumbv7em-none-eabihf` (verified by
    `cargo check -p dol-cas --target thumbv7em-none-eabihf --no-default-features`).

- **`ContentIndex` stub** (`lib/dol-cas/src/content_index.rs`,
  `feature = "std"`) per plan §7.6.
  - `HashMap<NodeId, [u8; 16]>` with `get` / `insert` / `invalidate` /
    `len` / `is_empty`. The bottom-up walker that populates this is
    deferred to M3 when `ExprArena` exists.

- **Phase 2 acceptance tests** (per plan §7.7) added inline:
  - `Option<StrId>` is 4 bytes; `size_of::<Cid<()>>() == 16`;
    `size_of::<Gid<_>>() == 32`.
  - `StringPool::intern`: same string from 16 threads → same `StrId`.
  - `to_cid()` is lazy (cache empty before first call, populated after)
    and cross-pool stable (two pools agree on the same input).
  - `StaticStringPool` round-trip, dedup, `SlotsExhausted` /
    `BytesExhausted` exhaustion paths.
  - `DynPool` and `StaticPool` typed push/get round-trip + capacity
    exhaustion.
  - 24 new tests; total `dol-cas` test count 24; workspace 127 → 151.

- **Cargo.toml.**
  - `dol-cas` now depends on `dol-core` with the `hash` feature
    explicitly enabled (xxHash3 + BLAKE3 chokepoints).
  - Adds `hashbrown` (workspace pin, default features off).
  - New optional `serde` feature that re-exports `dol-core/serde`.
  - `default = ["std"]` so the host build is `Send + Sync`-capable
    out of the box; bare-metal callers opt out with
    `--no-default-features`.

### Deferred to M3

- **Sharded `StringPool` interior.** The plan describes per-shard slot
  tables, which forces shard-index bits into `StrId` and breaks the
  simple `Lid::index() == slot_idx` contract. M3 will re-introduce
  sharding either via upper-bit encoding or by sharding only the
  lookup index while keeping slot/byte vectors global. The user-visible
  API is identical.
- **`impl PathSegment for Lid<StrTag>`** (plan §7.5). The
  `PathSegment::resolve` lifetime contract (`&'a str` borrowed from
  the resolver) cannot be satisfied while `StringPool` bytes live
  behind an `RwLock` — solving it requires either reshaping the trait
  (e.g. an associated `Resolved<'a>` type) or pulling in an
  append-only resolver crate. Both touch dol-core and fit better
  with the M3 lowering work where `ExprArena` will need the same
  lifetime shape. Until then, callers can manually resolve `StrId`s
  via `StringPool::get` and build `Path<Name>` from the resulting
  owned strings.

### v2 rewrite — Phase 1: `dol-core` foundation (`dol-rewrite-plan-v2.md` §6)

**M1 adds the new configuration / budget / hash chokepoints `dol-core`
provides to every other crate. The existing `path` and `strings::Name`
surfaces already match plan §6.4–§6.5; this PR layers the missing
pieces on top non-destructively.**

- **New `config` module** (`lib/dol-core/src/config.rs`) per plan §6.1.
  - `Config` bundles `BudgetConfig` + `PoolConfig` + `HashConfig` + `Profile`.
  - Three presets: `Config::standard()`, `Config::embedded()`, `Config::iot_min()`.
  - `Profile { Standard, Embedded, IotMin }` is the deployment-shape tag
    M2 will key `Pool` / `StaticStringPool` selection off of.
  - `PoolConfig::is_valid()` validates that `shard_count` is a non-zero
    power of two so the fast-modulo `hash & (shard_count - 1)` is correct.
  - `HashStrategy { Fast { seed }, Crypto, CryptoFull }` lets callers
    pick xxHash3 (in-process dedup) vs BLAKE3-128 / BLAKE3-256
    (cross-process stable) per call site.

- **New `budget` module** (`lib/dol-core/src/budget.rs`) per plan §6.2.
  - `Budget { depth, nodes, bytes }` is a flat `Copy` counter struct
    seeded from `BudgetConfig::from_config(&cfg)`.
  - `Budget::depth()` / `node()` / `bytes(n)` return `Result<(), BudgetExceeded>`
    with `checked_sub` so overflow is impossible.
  - `BudgetExceeded { Depth, Nodes, Bytes }` is the first-violation enum
    (impl `Display`, `core::error::Error` under `std`).
  - `Budget::leave_depth()` (saturating credit) lets symmetric tracers
    pair depth charge/credit; most monotonic traversals will ignore it.
  - Coexists with the legacy `policy::Budget` for the M0–M3 transition;
    the budget-gate `xtask` accepts either.

- **Extended `hash` module** per plan §6.3.
  - `fast64(bytes)`, `fast64_seeded(bytes, seed)`, `fast128(bytes)` —
    xxHash3-backed, non-cryptographic, in-process dedup only.
  - `content128(bytes)`, `content256(bytes)` — BLAKE3-backed,
    cross-process stable. `content128` is byte-prefix-consistent with
    `content256` (same property as the existing `hash128`/`hash256`).
  - Adds `xxhash-rust` to `dol-core` deps (gated behind the existing
    `hash` feature).
  - The legacy `hash32` / `hash128` / `hash256` / `Digest{32,128,256}`
    surface stays in place until M2 migrates `strings::interner` to
    `content128`.

- **Extended `PathSegment` trait** per plan §6.5.
  - New default method `content_id(&self, _: &Self::Resolver) -> Option<[u8; 16]>`
    returns `None` for inline segments (`Name`, etc.) and `None` for the
    existing `StrId` impl. M2's `Lid<StrTag>` impl will override it to
    return the cross-process stable BLAKE3-128 content address computed
    at intern time. Backward-compatible: existing impls do not need
    changes.

- **Phase 1 acceptance tests** (per plan §6.8) added inline:
  - `Config::{standard,embedded,iot_min}` presets produce non-zero
    budget/pool values and a valid (power-of-two) `shard_count`.
  - `Budget::depth()` / `node()` / `bytes(n)` return the matching
    `BudgetExceeded` variant exactly when the corresponding cap is hit.
  - `fast64`, `fast64_seeded`, `fast128`, `content128`, `content256`:
    same input → same output; different inputs → different output (with
    cryptographic / probabilistic confidence as appropriate). `content128`
    is verified to be a strict prefix of `content256`.
  - 19 new unit tests; total `dol-core` test count rises from 67 → 86.

- **Cargo metadata.**
  - `dol-core` `hash` feature now turns on both `blake3` and `xxhash-rust`.

### Explicitly NOT in this PR

The §6.6 `types` reshape (consolidating `data_type` + `literal` + `value`
into a single `types` module with `Literal<'a>` lifetime-borrowed
variants) and the §6.7 `span` / `diagnostic` trim. The existing modules
already implement compatible surfaces and downstream crates (`dol-cas`,
`dol-ir`) do not need the consolidation to land; reshaping is deferred
to a follow-up so PR #2 stays surgically scoped to the foundation
additions that M2 / M3 *do* need.

### v2 rewrite — Phase 0: workspace scaffolding (`dol-rewrite-plan-v2.md` §5)

**M0 sets the v2 workspace skeleton; M1+ content lands in subsequent PRs.**

- **Workspace restructure.** The 9-crate v1 layout collapses to a 5-crate
  library workspace plus an umbrella per `dol-rewrite-plan-v2.md` §3:

  | Before                                      | After                             |
  |---------------------------------------------|-----------------------------------|
  | `lib/core` · `dol-core`                     | `lib/dol-core` · `dol-core`       |
  | *(none)*                                    | `lib/dol-cas` · `dol-cas` (empty) |
  | `lib/expr` + `lib/schema` + `lib/command`   | `lib/dol-ir` · `dol-ir` (empty)   |
  | `lib/wire` · `dol-wire`                     | `lib/dol-wire` · `dol-wire` (thinned) |
  | `lib/query` · `dol-query`                   | `lib/dol-query` · `dol-query` (thinned) |
  | `lib/stream` + `lib/pipeline`               | absorbed into `dol-ir` (M3+, plan §3.3) |
  | `lib/dol` · `dol`                           | `dol/` · `dol` (thinned)          |
  | `tools/check`, `tools/fmt`                  | thinned stubs against `dol-core`  |

- **Crates excluded from the workspace** but preserved on disk as a
  reference to mine from during M1–M5: `lib/expr`, `lib/schema`,
  `lib/command`, `lib/stream`, `lib/pipeline`. Likewise the legacy source
  of the thinned crates is preserved under `_legacy_src/` and
  `_legacy_tests/` subfolders in each survivor.

- **New empty scaffold crates.** `lib/dol-cas` (M2 target — content
  addressing, handles, pools, `StringPool`) and `lib/dol-ir` (M3 target —
  `ExprArena`, schema catalog, `Operation`, `Program`, `Backend`) are
  `#![no_std]` placeholders that compile clean today.

- **Workspace dependency pins** added per `dol-rewrite-plan-v2.md` §5.1:
  `xxhash-rust`, `proptest`, `criterion`, `insta`. The pre-existing
  `blake3`, `smallvec`, `serde`, `postcard`, `serde_json`, `hashbrown`,
  `bytemuck`, `defmt` pins are kept; legacy entries are retained so the
  on-disk reference crates remain build-able under a temporary
  side-workspace during M1–M3 reads.

- **`xtask` rewritten** to the five subcommands listed in
  `dol-rewrite-plan-v2.md` §5.2:

  | command         | M0 status                                         |
  |-----------------|---------------------------------------------------|
  | `budget-gate`   | Real impl carried over from v1; scans every `pub fn (walk|visit|decode|lower|content_hash)_*` in `lib/`. |
  | `size-check`    | Exit-0 stub; real impl in M3 once `ExprNode` exists. |
  | `dag-check`     | Exit-0 stub; real impl when more than scaffold crates have content. |
  | `no-std-check`  | Exit-0 stub; CI invokes `cargo check --target thumbv7em-none-eabihf` directly. |
  | `size-report`   | Exit-0 stub; real impl in M6 with the IoT preset. |

- **CI** (`.github/workflows/ci.yml`) trimmed of jobs that depended on
  excluded crates and re-shaped around M0:
  - **kept:** `fmt`, `clippy`, `check`, `test` (with budget-gate step),
    `msrv`, `deny`.
  - **added:** `xtask` job runs the four new M0 stub commands;
    `no-std` job builds `dol-core` / `dol-cas` / `dol-ir` for
    `thumbv7em-none-eabihf` per `dol-rewrite-plan-v2.md` §5.3.
  - **removed:** `test-features`, `dol-core-features`, `iot-min-check`,
    `xtask-gates`, `cross-compile`, `fuzz-smoke`, `udeps`, and the wire
    round-trip / tag-table gates — all depended on excluded crates or
    feature surfaces that no longer exist. They graduate back layer by
    layer as the v2 content lands.

- **Pre-existing dol-core bugs** uncovered by the workspace cleanup and
  patched minimally to get a green M0 build (M1 rewrites the module
  anyway):
  - `pub mod content_addressing` removed from `lib/dol-core/src/lib.rs`
    (referenced `crate::budget`/`crate::id` paths that never existed,
    plus const-generic expressions that require nightly).
  - Duplicate `hash128`/`hash256` definitions in `lib/dol-core/src/hash.rs`
    gated out with `#[cfg(any())]`.
  - `strings::interner` and the `PathSegment for StrId` impl gated behind
    the existing `hash` feature.
  - Broken intra-doc links in `lib/dol-core/src/lib.rs` (`diag`, `id`)
    redirected to their real module paths.
  - `hash` is now a default feature of `dol-core` (matches what every
    pre-PR consumer was already enabling).

- **Workspace serde pin** changed to
  `serde = { default-features = false, features = ["derive", "alloc"] }`
  (was: default features on). Adding `alloc` is the no_std-friendly
  equivalent and unblocks dol-core's `String: Serialize` usage.

### Explicitly NOT in this PR

No M1+ content: no `Name`, no `PathSegment`, no `StringPool`, no
`ExprArena` reshape, no backends. Those land in PR #2 onwards as M1
(`dol-core`), M2 (`dol-cas`), M3 (`dol-ir` + first backend), and so on.

### Crate-boundary review follow-up (PR 11)

- Re-extracted **`dol-stream`** and **`dol-pipeline`** as standalone crates.
  `dol-query` is now only the fluent query DSL builders.
- `dol-command::query_extensions::{stream,pipeline}` now depend on
  `dol-stream` and `dol-pipeline` directly (instead of
  `dol-query::{stream,pipeline}`).
- Moved schema handle/constraint primitives (`SchemaRef`, `SchemaId`,
  `CatalogId`, `TypeBody`, `ComputedKind`, `RefAction`, `RelationRef`,
  `EntityConstraint`) out of `dol-core` and back into `dol-schema`.
- Removed `dol-command`'s `schema` feature gate; `dol-schema` is now a
  normal dependency of `dol-command`.
- The `dol` umbrella now exposes `stream` and `pipeline` features/crate
  re-exports again, and `query` includes them for compatibility.

### Dependency-graph alignment (PR 7 + PR 8a + PR 9 + PR 10)

All four DAG violations identified in the v2-layout audit are now fixed.
The workspace dependency graph matches what's documented at the top of
the root `Cargo.toml`: `core` ← `expr` ← `schema` ← `command`,
`query` ← `core`/`expr`/gated `schema` (no `command` edge), and
`command` ← gated `query` (replacing the historical reverse edge).

#### `dol-query → dol-command` edge inverted (PR 10)

The honest path described in the previous CHANGELOG entry — design a
query-native plan that `dol-command` adapts — turned out not to be the
cleanest option. The *invert-the-dependency* path is simpler and ends
up at the same v2 DAG shape with fewer moving pieces:

- `dol-query` is now a **pure data crate**. It owns the DSL builder
  structs (`GetQuery`, `InsertQuery`, `UpdateQuery`, `DeleteQuery`,
  `UpsertQuery`), the `JoinClause`/`JoinKind` types, and the streaming
  / pipeline data (`WindowSpec`, `TimeSeriesOp`, `Sample`, `Graph`,
  `Node`, …). The builder structs' fields and `Query.{name, namespace,
  field_names}` are now `pub` so cross-crate lowering can read them
  without going through accessors.
- The lowering that turns a builder into a `dol_command::Program`
  (formerly `try_build` methods on each builder) now lives in
  `dol_command::lower_query`, behind a new default-on `query` feature
  on `dol-command`. Free functions: `lower_get`, `lower_insert`,
  `lower_update`, `lower_delete`, `lower_upsert`. A `BuildProgram`
  trait re-creates the historical chained `builder.try_build()`
  ergonomics — callers add `use dol_command::lower_query::BuildProgram;`
  and the old call patterns work unchanged.
- `BuildError` (formerly `dol_query::BuildError`) moved to
  `dol_command::lower_query::BuildError`. The variants are unchanged
  (`Filter`, `Having`, `Projection`, `GroupBy`, `OrderBy`, `SetValue`,
  `JoinOn`).
- The typed `OperationExtension` payload wrappers
  (`WindowPayload`, `TimeSeriesPayload`, `SamplePayload`,
  `PipelinePayload`) and their stable `Symbol` constants
  (`WINDOW_SYMBOL`, `TIMESERIES_SYMBOL`, `IOT_SAMPLE_SYMBOL`,
  pipeline `EXTENSION_SYMBOL`) moved to
  `dol_command::query_extensions::{stream, pipeline}`. The wrappers
  `impl ExtensionPayload` natively (the trait lives in `dol-command`),
  so `OperationExtension::from_payload(&p).into_operation()` works
  without any back-edge from `dol-query`.
- `dol-query`'s prelude no longer re-exports anything from
  `dol-command`. Consumers of the IR helpers and types
  (`define_entity`, `tx_begin`, `IsolationLevel`, `PolicyScope`,
  `Privilege`, `SchemaBinding`, …) import directly from
  `dol_command::builders` and `dol_command::prelude`, where they
  always lived.
- `dol-query`'s `dol-command` runtime dep is gone; it's now a
  *dev-dependency* with the `query` feature, used by the integration
  tests in `lib/query/tests/integration.rs` (moved out of
  `lib/query/src/tests.rs`; an inner `#[cfg(test)] mod tests;` would
  have given the test binary a different `dol_query::GetQuery` type
  than the one `dol-command`'s `BuildProgram` impl was compiled
  against).
- `cargo tree -p dol-query --edges normal` confirms zero
  `dol-command` references. With the new `dol-command --features
  query`, the graph is `command → query` only.
- `dol`'s `query` feature now forwards `dol-command/query` so umbrella
  consumers see the lowering surface.

### Done previously in this cycle

#### `dol-command`: `dol-schema` is now a default-on `schema` feature (PR 9)

The blocker called out in the previous PR ("`SchemaRef` is everywhere in
every Operation, and `Program.schema_catalog: SchemaCatalog` is a hard
field — gating it would cascade through every backend/wire/check") is
resolved by **relocating the small handle types into `dol-core`**:

- New `dol_core::schema` module hosts the zero-dep handle / classifier
  types: `SchemaRef`, `SchemaId`, `CatalogId`, `TypeBody`,
  `ComputedKind`, `RefAction`, `RelationRef`, `EntityConstraint`. These
  are pure newtypes and small enums — they don't pull in any of the
  catalog *storage* (`SchemaCatalog`, `Entity`, `Field`, `DataType`)
  that justifies `dol-schema` existing as a separate crate.
- `dol-schema`'s `schema_ref.rs`, `type_body.rs`, and `constraint.rs`
  are now thin re-export shims over `dol_core::schema`. The names
  `dol_schema::SchemaRef` / `TypeBody` / `RefAction` / etc. continue to
  resolve, so downstream callers see no change.
- `dol-command` imports the handles from `dol-core::schema` everywhere
  they appear in the IR (`Target`, `SchemaOp`, `FieldDef`,
  `prelude.rs`). With handles relocated, the *only* remaining
  `dol-schema` usages in `dol-command` are the catalog *storage*
  (`Program.schema_catalog`, `define_from_entity` builder) and the
  prelude re-exports of `SchemaCatalog`/`CatalogEntry`/`TypeEntry`.
- `dol-schema` is now an optional `schema` feature on `dol-command`,
  default-on. Without it: `Program.schema_catalog` field, the
  `Program::with_catalog` constructor, the `ProgramRef.schema_catalog`
  field, the `ExtendError::CatalogConflict` variant, the
  `define_from_entity` builder, and the prelude re-exports of catalog
  storage types are all `#[cfg]`-gated out.
- `cargo tree -p dol-command --no-default-features` confirms
  `dol-schema` is no longer in the dependency graph; only `dol-core`,
  `dol-expr`, and `smallvec`.

#### `dol-schema`: `dol-expr` is now a default-on `expr` feature (PR 7)

- `dol-schema` previously hard-depended on `dol-expr` for the
  `dol_expr::ids::StrId` type used in `TypeEntry` and
  `CatalogEntry::{Type, Extension}`. The dep is now optional and gated
  by a new `expr` feature (on by default).
- With `--no-default-features`, `dol-schema` builds without
  `dol-expr` in its dep tree (verified with `cargo tree`). The
  `TypeEntry` struct and the `CatalogEntry::{Type, Extension}` variants
  are gated out; only `CatalogEntry::Entity` remains.
- `dol-wire` and `dol-command` opt into `dol-schema`'s `expr` feature
  explicitly (they already depend on `dol-expr` directly), so the API
  surface they see is unchanged.
- The `dol` umbrella's `schema` feature now forwards `dol-schema?/expr`
  so umbrella consumers also see the full catalog API.

#### `dol-query`: `dol-schema` is now a default-on `schema` feature (PR 8a)

- `dol-query` previously hard-depended on `dol-schema` for
  `Query::from(&Entity)`. The dep is now optional and gated by a new
  `schema` feature (on by default).
- With `--no-default-features`, `dol-query` builds without referencing
  `dol-schema` directly. `Query::from(&str)` works unconditionally;
  `Query::from(&Entity)` requires the `schema` feature.
- The doctest in `dol_query::lib`'s top-level docs is now split: the
  `&str` half is unconditional, the `&Entity` half is gated behind
  `#[cfg(feature = "schema")]`.

### Crate rename: `dol-ir` → `dol-command`

The IR crate has been renamed to **`dol-command`** to better reflect its
scope: it is the universal command-language layer (Operation, Program,
Backend, capability checks, plus the DDL/ACL/Tx/storage builders), not
just an "intermediate representation".

The directory moved from `lib/ir/` to `lib/command/`. The umbrella's
`ir` feature has been renamed to `command`, and the umbrella now
re-exports the crate as `dol::command`.

#### Migration

- `dol_ir::*` → `dol_command::*` everywhere.
- `dol::ir::*` (umbrella) → `dol::command::*`.
- `dol = { features = ["ir"] }` → `dol = { features = ["command"] }`.
- `Cargo.toml`: `dol-ir = { workspace = true }` → `dol-command = { workspace = true }`.

### Builder helpers moved: `dol-query` → `dol-command::builders`

The DDL (`define_entity`, `define_index`, …), ACL/Tx (`grant`, `revoke`,
`define_policy`, `tx_begin`, `tx_atomic`, …), and storage (`get_blob`,
`put_blob`, `read_file`, `write_file`, …) helpers have been moved from
`dol-query` into the new `dol_command::builders` module.

These helpers construct IR directly without using the fluent query DSL,
so they belong with the IR layer; `dol-query` keeps only the actual
query builders (`GetQuery`, `InsertQuery`, `UpdateQuery`,
`DeleteQuery`, `UpsertQuery`).

**Note:** `dol_query::prelude` no longer re-exports builders from
`dol-command`. Import them directly from `dol_command::builders`.

#### Migration

- `dol_query::define_entity` → `dol_command::builders::define_entity`.
- `dol_query::ddl::*` / `dol_query::control::*` / `dol_query::storage::*`
  modules — removed; import from `dol_command::builders::{ddl, control,
  storage}` (or use the flat re-exports in `dol_command::builders`).

### Crate re-extraction: `dol-stream` + `dol-pipeline` (standalone again)

`dol-stream` and `dol-pipeline` are once again standalone workspace
crates (they were briefly absorbed into `dol-query` and have now been
re-extracted). Streaming / time-series / IoT types live in `dol-stream`;
pipeline DAG types live in `dol-pipeline`. Both integrate with
`dol_command::operation::Operation` via the `Extension` seam through
typed payloads in `dol_command::query_extensions`.

#### Migration

- `dol_query::stream::WindowSpec` → `dol_stream::WindowSpec` (and
  similarly for every other type).
- `dol_query::pipeline::Graph` → `dol_pipeline::Graph`.
- The `dol` umbrella re-exports `dol::stream` and `dol::pipeline`
  behind the `stream` and `pipeline` features respectively.
- The `iot-min` preset now pulls `stream` instead of `query`.

### Removed

- **`dol-stream`** and **`dol-pipeline`** workspace members — content
  rehomed into `dol-query` (see above).
- **`dol_command::schema_ref`** and **`dol_command::schema_catalog`**
  modules (formerly under `dol_ir`) — moved to `dol_schema`
  (`SchemaRef`, `SchemaId`, `CatalogId`, `SchemaCatalog`,
  `CatalogEntry`, `TypeEntry`, `TypeBody`).
- **`dol_command`'s flat `pub use` re-exports** at the crate root —
  callers now import from the source module (e.g.
  `dol_command::operation::Operation` instead of
  `dol_command::Operation`, `dol_command::target::Symbol` instead of
  `dol_command::Symbol`, `dol_schema::EntityConstraint` instead of
  `dol_command::EntityConstraint`). The `dol_command::prelude`
  re-export is unchanged for callers who want the flat surface.
- **`dol_command::store` module** (`KvStore`, `Catalog`, `KvError`) —
  premature abstraction: the KV-specific trait shape does not apply to
  all backend families (SQL, graph, document, …). Will be reintroduced
  when a concrete backend lands and the trait surface is informed by
  real usage.

### Operator / expression tier lock-down

A principled cut of what earns a slot in `BinOp` / `UnaryOp` / `ExprOp`
versus what belongs in `ExprOp::Func`. See the four-tier rule baked into
the head of `lib/expr/src/expr.rs` for the criteria. Every construct
goes in exactly one tier.

The headline driver: `ContainedBy` (`<@`) semantically conflicted with
`IN` (`x <@ array_of(a,b,c)` and `x IN (a,b,c)` spell the same
membership test for scalar `x`). Removing `<@` from the IR resolves
the conflict without losing expressiveness — `<@` is exactly
`contains(b, a)`.

#### Removed (wire-format break — acceptable per project stage)

- **`BinOp::Arrow` (tag 22)** and **`BinOp::LongArrow` (tag 23)** — JSON
  path step (`->`, `->>`) is not universally infix (Postgres / MySQL
  only). Use the existing `JSON_GET` / `JSON_GET_TEXT` functions
  instead; backends that have native infix render the call inline.
- **`ExprOp::IsNull` (opcode 18)** — duplicated `UnaryOp::IsNull`. Only
  the unary form remains; the standalone opcode is gone, and the
  following opcodes shift down by one (`Between` 19→18, `Query` 20→19,
  `Insert` 21→20, `Update` 22→21, `Delete` 23→22, `Upsert` 24→23).
  `ExprArena::alloc_is_null`, `ExprNode::is_null`, and
  `ExprNode::as_is_null` are deleted accordingly.
- **`tree::op::registry`**: `OpContains`, `OpContainedBy`, `OpOverlap`,
  `OpRegexMatch`, `OpRegexMatchInsensitive`, `OpGlob` deleted. None of
  these are universally infix across backends and so do not qualify
  for `BinOp` slots.
- **`tree::op::meta::OpCategory::Collection`** variant removed (had no
  members after the deletions above).
- **`OpDef::CONTAINS` / `CONTAINED_BY` / `OVERLAP` / `REGEX_MATCH` /
  `REGEX_MATCH_INSENSITIVE` / `GLOB`** name constants removed.

#### Added

- **`BinOp::IsDistinctFrom` (tag 22)** and **`BinOp::IsNotDistinctFrom`
  (tag 23)** — promoted from the tree-only `OpIsDistinctFrom` /
  `OpIsNotDistinctFrom` ZSTs, which previously had no real `BinOp`
  mapping and silently fell through to `BinOp::Eq` in `lower_binop`.
  Now they round-trip correctly through wire and lowering.
- **`tree::func::registry`**: `RegexMatch` (`REGEX_MATCH`),
  `RegexImatch` (`REGEX_IMATCH`), `GlobMatch` (`GLOB_MATCH`),
  `Overlaps` (`OVERLAPS`). `Contains` already existed and now also
  serves the collection containment role.
- **DSL methods on `Expr`**: `contains(other)`, `overlaps(other)`,
  `glob_match(other)`. `regex_match` and `regex_match_insensitive`
  retained but now lower through `Func` (no behavior change for
  callers — only the IR shape moves from `Bin` to `Func`).

#### Tag-table stability

After this cut, `BinOp::as_u16`, `UnaryOp::as_u16`, and
`ExprOp::from_u8` tag tables are **frozen append-only**. New variants
must use the next free tag; never renumber. The four-tier rule is
documented at the head of `lib/expr/src/expr.rs` so future additions
get the question right the first time.

#### Deferred (now landed in this branch — see new sections below)

The companion `In` (collapsing `InList` + `InSub`) and `Composite`
(collapsing `ObjectLit` + `ArrayLit`) opcode unifications, plus the
xtask tag-table drift gate, have all landed in this same branch — see
"### Phase 2 follow-ups (deferred items, now landed)" below for the
detailed entry. No item that was tagged "deferred" in this section is
outstanding.

### Phase 2 follow-ups (deferred items, now landed)

This entry lands every item the prior PR explicitly held over for a
follow-up. Wire format breaks from this section are flagged inline.

#### Operator / expression IR — opcode collapse (wire-format break)

- **`ExprOp::In` (new tag 14)** — single structural opcode replacing
  `ExprOp::InList` and `ExprOp::InSub`. The node carries
  `(probe: NodeId, collection: NodeId)`; the *collection*'s own opcode
  discriminates the form (`Composite` for `(a, b, c)` row form,
  `Query` for `(SELECT …)` subquery form, `Param` for parameter form,
  `Field` for field-of-array). Replaces the old separate-opcode design
  and removes the `InListNode` side-pool entirely.
  - `ExprNode::in_list(InListId)` and `ExprNode::in_sub(NodeId,
    NodeId)` deleted; `ExprNode::in_(probe, collection)` added.
  - `ExprNode::as_in_list` / `as_in_sub` deleted; `ExprNode::as_in`
    added (returns `(probe, collection)`).
  - `ExprArena::alloc_in_list_ref` / `alloc_in_sub` deleted;
    `ExprArena::alloc_in(probe, collection)` added.
  - DSL: `Expr::in_list(...)` is unchanged at the AST level; it now
    lowers to `In { probe, collection: Composite::Array(list) }`.
  - `InListId`, `InListNode` types deleted.
- **`ExprOp::Composite` (new tag 5)** — single structural opcode
  replacing `ExprOp::ObjectLit` and `ExprOp::ArrayLit`, with a
  three-arm `CompositeKind` discriminant (`Array`, `Object`, `Tuple`).
  `Tuple` is added on day one to cleanly back the row form of
  `IN (a, b, c)`.
  - `CompositeNode { kind: CompositeKind, items: SmallVec<[(Option<StrId>,
    NodeId); 4]> }` replaces the old `ObjLitNode` / `ArrayLitNode`.
    Object items carry `Some(StrId)` keys; Array / Tuple items carry
    `None`.
  - `ExprArena::obj_lits` / `array_lits` / `in_lists` pools collapsed
    into one `composites` pool. `Capacity::obj_lits` / `array_lits` /
    `in_lists` fields deleted; `Capacity::composites` added.
  - `ObjLitId`, `ArrayLitId`, `ObjLitNode`, `ArrayLitNode` types
    deleted; `CompositeId`, `CompositeNode`, `CompositeKind` added.
- **Tag table renumbered** — every opcode after `Lit` shifts down. The
  new dense table is: `Nop=0`, `Namespace=1`, `Field=2`, `Param=3`,
  `Lit=4`, `Composite=5`, `Bin=6`, `Una=7`, `Func=8`, `Agg=9`,
  `Window=10`, `Cast=11`, `Case=12`, `Alias=13`, `In=14`, `Exists=15`,
  `Between=16`, `Query=17`, `Insert=18`, `Update=19`, `Delete=20`,
  `Upsert=21` (22 entries; previous build had 24). The four-tier-rule
  head doc in `lib/expr/src/expr.rs` reflects the new layout.

#### xtask tag-table drift gate

- **`xtask tag-table-gate`** subcommand: walks frozen `(name, tag)`
  fixtures for `BinOp`, `UnaryOp`, and `ExprOp`, calls
  `BinOp::try_from_u16` / `UnaryOp::try_from_u16` / `ExprOp::from_u8`
  for every entry and asserts the variant matches, then asserts the
  *next* tag past the last fixture entry returns `None`/`Err`. Catches
  accidental enum additions or renumbers without a matching fixture
  update; failure message tells the dev to update both in the same PR.
- **CI**: new "Tag-table drift gate" step parallel to "Budget-gate" in
  `.github/workflows/ci.yml`.
- **Fixture**: the snapshot lives in code (`xtask/src/tag_table.rs`)
  rather than a JSON file so it compiles with the same enum types it
  validates — the gate cannot drift from the tag types it gates.

#### `ExprNode` — bulk POD wire format (wire-format break)

- The per-`ExprNode` wire form switched from a per-opcode varint
  encoding (3-13 bytes per node) to a fixed 16-byte LE field-by-field
  encoding (`op` (1 B), `flags` (1 B), `aux` (2 B LE), `a`/`b`/`c`
  (4 B LE each)). The new form is byte-for-byte identical to
  `bytemuck::cast_slice::<ExprNode, u8>` on little-endian hosts (the
  overwhelming majority), so encoding the nodes vector reduces to a
  single contiguous write — the bulk-POD fast path the previous PR
  flagged.
- Decode validates each node's `op` byte against `ExprOp::from_u8` so
  a corrupted byte stream surfaces a clean `DecodeError::InvalidVariant`
  rather than an unchecked enum.
- `Encode for ExprArena` no longer special-cases per-opcode payloads;
  it just length-prefixes the nodes slice and writes `N × 16` bytes.

#### `ExprArena::intern_node` — structural dedup

- New `ExprArena::intern_node(node) -> NodeId` interns the 16 B
  packed `ExprNode` by structural identity (BLAKE3-free; `ExprNode`
  derives `Hash` over its bytes). Returns the existing `NodeId` for
  any structurally-equal node already in the pool.
- The dedup index is *opt-in*: it is `None` until the first
  `intern_node` call. The bulk single-shot `ExprArena::alloc` path
  stays unchanged so single-shot lowering does not pay the hash cost.
- The index is rebuilt-on-demand and skipped by `Serialize` (it's
  recoverable from the nodes slice).

#### `lower_inner` — iterative left-spine BinaryOp walker

- The `Expr::BinaryOp` arm in `lower_inner` now walks the left spine
  iteratively (heap-allocated `Vec<(BinOp, &Expr)>`) before lowering
  the bottommost left operand and building bin nodes bottom-up.
  Adversarial inputs (deep `a | b | c | …` chains) no longer scale
  host-stack consumption with chain length — only the right operands
  and the bottommost left consume host frames, both bounded by the
  individual right operand's depth (typically 2-3 frames).
- The depth contract is unchanged: the spine layer count is checked
  against `Limits::max_depth` explicitly; a chain longer than the cap
  surfaces as `LowerError::DepthExceeded` exactly as before.
- The `deep_or_chain_succeeds_with_ample_depth_cap` test was bumped
  from chain depth 64 (the previous recursive limit) to 8 192 (two
  orders of magnitude higher), pinned at the `Box<Expr>::Drop` host
  ceiling rather than the lowerer's. A regression that reintroduces
  recursion into the left spine would overflow the default test stack
  on this input.

#### Test corpus

- `lib/wire/tests/expr_node_roundtrip.rs`: `in_list_round_trips` and
  `in_sub_round_trips` replaced by `in_round_trips` (single `In`
  opcode). `object_lit_round_trips` and `array_lit_round_trips`
  replaced by `composite_round_trips`. Three new tests pin the
  fixed-16-byte wire size, byte-identity with the in-memory POD on
  LE hosts, the bulk arena round-trip (1 000 nodes), and the
  invalid-opcode rejection path.
- `lib/expr/src/arena.rs`: four new tests covering `intern_node` —
  structural-equality dedup of leaf nodes, `alloc`-then-`intern`
  index seeding, `intern`-then-`alloc` index sync, and
  `(a + 1) * (a + 1)` subtree dedup.
- `xtask::tag_table::tests`: live-vs-fixture cross-check tests for
  every variant of `BinOp`, `UnaryOp`, `ExprOp`, plus
  next-tag-is-none assertions.

#### Still deferred

- **`ExprArena` `Storage`-generic** (CHANGELOG line ~150 in the v2
  cut). Parameterising the nodes pool *and* every side pool over
  `dol_core::storage::Storage<T>` requires turning every `alloc_*`
  into a fallible `Result<…, BackpressureError>` and threading 12+
  default type parameters through every call site (query / ir /
  wire). Even with the `Vec<T>` defaults the plan suggested, the
  cascade through `query::*::try_build`, `ir::Operation::*`, and
  every `lower_*` helper makes this a focused refactor in its own
  right with zero functional benefit for the current `Vec` backend.
  Folding it into this branch alongside the IR opcode collapse and
  the lowerer rewrite would obscure the contract changes; it lands
  cleanly once a concrete bounded `Storage` impl (e.g. an MCU
  backend) needs the surface. All other deferred items from the
  prior PRs have landed in this branch.

### Phase 2 — finalize v2 (single PR, no shims)

This entry rounds out the v2 cut started in 0.2.0. No new package versions
are bumped; this is an additive tightening of the contracts already
documented under "0.2.0" below.

#### Removed

- **`serde::Deserialize` is gone from every in-memory IR/AST/expr/schema
  type** (`Operation`, `Program`, `ExprArena`, `Interner`, `CompactName`,
  `Id<Tag>`, `Field`, `DataType`, `Value`, `Literal`, every leaf in
  `core::{datetime, geo, network, numeric, binary}`, every IR verb
  payload). `Serialize` is retained for JSON debug output. v2 wire-in
  goes through `dol-wire::Decode` exclusively.
- **`dol_wire::program::decode_postcard` and `decode_json` deleted.**
  Replaced by `dol_wire::program::decode` (drives `Decode` end-to-end
  with `&mut Budget`). Generic serde-based `dol_wire::postcard::decode`
  and `dol_wire::json::decode` shims also deleted.
- **`dol_core::descriptor` deprecated alias removed** (`data_type` is the
  canonical name).
- **Per-crate `tests/serde_roundtrip.rs` deleted** — universal-`serde`
  round-trip is no longer the v2 contract; replaced by the
  `dol-wire`-internal `Encode`/`Decode` test corpus and a JSON-output
  smoke test on the IR side.

#### Added

- **`dol_expr::ExprNode` is now the 16-byte `bytemuck::Pod` packed node**
  (the previous variant enum is gone, completing the v2 cut-over per
  `docs/v2_plan.md` §35). The user-visible identifier
  `dol_expr::ExprNode` is preserved everywhere downstream — what
  changed is the *layout*: an opcode (`u8`), a `flags` byte, a `u16`
  `aux`, and three `u32` operand handles (`a`, `b`, `c`). The
  scaffolding `dol_expr::packed::{PackedNode, PackedOp, walk_iter}`
  module is gone; opcodes live on the new `dol_expr::ExprOp`
  (re-exported from the crate root) and `walk_iter` is replaced by
  `ExprNode::child_node_ids()` (yields the up-to-three child
  `NodeId`s for any recursive opcode).
  - **`ExprOp` discriminator** — covers all 26 shapes in a stable,
    append-only byte tag space (≤ 256 today). Operator families
    collapse onto shared opcodes: every binary operator is
    `ExprOp::Bin` (with `BinOp::as_u16` in `aux`); every unary
    operator is `ExprOp::Una` (with `UnaryOp::as_u16` in `aux`).
  - **Typed constructors and `as_*` accessors** — `ExprNode::bin(op,
    lhs, rhs)`, `ExprNode::field(id)`, …, mirrored by
    `node.as_bin() -> Option<(BinOp, NodeId, NodeId)>`,
    `node.as_field() -> Option<FieldId>`, … Backends never reach into
    raw `a`/`b`/`c`. Mirrored on `ExprArena` as `alloc_bin`,
    `alloc_una`, `alloc_field_ref`, `alloc_param`, …, so call sites
    stay readable.
  - **`BinOp::as_u16` / `BinOp::try_from_u16` and the same on
    `UnaryOp`** — single canonical mapping for the wire format and
    `aux` encoding; round-trip-tested for every variant so backends
    can't silently drift.
  - **`ArrayLitNode` + `ArrayLitId` side pool** — the only inline
    payload on the previous enum (`ExprNode::ArrayLit(SmallVec<[NodeId;
    4]>)`) moved out of the node into its own pool, accessed via
    `ExprArena::{alloc_array_lit, get_array_lit}`. This is what makes
    the 16-byte budget achievable.
  - **Wire format for `ExprNode` is new** (varint opcode +
    opcode-aware fields, byte-stable across hosts and dense for leaf
    nodes; smaller than the previous variant-tag encoding for most
    workloads). Because there are no external dependents this is fine,
    but old wire bytes will not decode against the new build.
  - **`xtask size` budget** — `ExprNode` now has a 16 B budget (down
    from 32 B); the test corpus and the
    `dol_expr::expr::size_tests::expr_node_is_16_bytes` unit test both
    enforce it.
  - **Iterative-traversal helpers** —
    `ExprNode::child_node_ids()` yields the recursive children for
    every opcode (Bin: a, b; Una: a; Agg: b; Cast: a; Alias: a;
    In: a, b; Exists: a; IsNull: a; Between: a, b, c). Side-pool
    referents (Field, Func, Case, Window, Composite,
    Query, Insert, Update, Delete, Upsert) yield empty
    here — their traversal goes through the arena's pool tables.
  - *Deferred items now landed in the same branch (see "Phase 2
    follow-ups (deferred items, now landed)" above):* bulk POD wire
    encoding (`bytemuck::cast_slice` byte-identity for the nodes
    vector on LE hosts via the fixed-16-byte `ExprNode` wire form);
    structural hashing/dedup of `ExprNode`s
    (`ExprArena::intern_node`); iterative left-spine walker for
    `Expr::BinaryOp` lowering (host stack no longer scales with chain
    length; the production guard remains the `Limits::max_depth`
    cap).
- **`dol_ir::store::{KvStore, Catalog, KvError}`** — backend trait
  surface per `docs/v2_plan.md` §71-72. Both traits thread
  `&mut Budget`, return `Result<…, KvError<E>>` with `Budget`,
  `NotFound`, and `Backend(E)` variants, and stay `no_std + alloc`-clean.
  `KvStore::Value: AsRef<[u8]>` lets MCU backends return stack arrays
  and host backends return `Vec<u8>` with no impedance mismatch.
  `Catalog` persists a [`SchemaCatalog`] atomically — concrete impls
  layer wire-encode + signature verification on top of a `KvStore`.
- **`dol_wire::signed::{Signed, SignedError}`** — `Signed<T>` envelope
  per `docs/v2_plan.md` §56. Wraps any `[u8]` payload in
  `WireHeader || payload_len || payload || sig_len || sig` and signs
  over the header + length-prefixed payload, so flipping the version
  field invalidates the signature. Generic over the workspace's
  existing `dol_core::sign::{Signer, Verifier}` traits — algorithm-
  agnostic; Ed25519 on hosts, HMAC-SHA-256 on MCUs.
  `Signed::open` is zero-allocation; `Signed::seal` allocates exactly
  once.
- *(Superseded by the `ExprNode` packed cut above.)* The earlier
  scaffolding `dol_expr::packed::{PackedNode, PackedOp, walk_iter}`
  has been folded into and replaced by the new `ExprNode` / `ExprOp` /
  `ExprNode::child_node_ids()` API; the `packed` module no longer
  exists.
- **`.github/workflows/nightly-sanitizers.yml`** — daily Miri
  (`-Zmiri-strict-provenance -Zmiri-symbolic-alignment-check`) and
  AddressSanitizer jobs over `dol-core`, `dol-expr`, and `dol-wire`,
  per `docs/v2_plan.md` §78. Runs at 04:17 UTC nightly,
  `workflow_dispatch`-able, and opt-in on PRs via the
  `nightly-sanitizers` label so the default review loop stays fast.

- **`dol_check::sarif::to_sarif`** — SARIF v2.1.0 emitter for
  `dol-check` diagnostics, per `docs/v2_plan.md` §68 (*"`check` runs
  every validator in one pass and emits SARIF (good DX in IDEs)"*).
  Hand-written JSON (no `serde_json` dependency, `no_std + alloc`-clean)
  that maps each `Diagnostic` to a SARIF `result`: `ruleId` is the
  canonical `DOLNNNN` form of the `ErrorCode`, `level` is
  `error`/`warning`/`note` (`Severity::Lint` folds into `warning`,
  matching how clippy is rendered), and physical locations carry the
  span's byte offset/length under the synthetic `dol-file://{file_id}`
  URI scheme. Output is consumable by VS Code (SARIF Viewer
  extension), GitHub Code Scanning, IntelliJ, and Azure DevOps.
- **`xtask ci` super-task** — local mirror of
  `.github/workflows/ci.yml` per `docs/v2_plan.md` §78
  (*"`xtask ci` runs the exact same gates locally as in CI"*). Runs
  `fmt`, `clippy -D warnings`, `check`, `test`, the `--no-default-features`
  doctest gate, `budget-gate`, `size`, `nostd`, `readme`, and the
  cross-compile `mcu` gate in one command. Pass `--quick` to skip the
  `mcu` step on hosts without the embedded targets installed. Fails
  fast on the first failing step. Third-party-tool gates
  (`cargo-deny`, `cargo-udeps`, `miri`, `cargo-fuzz`) stay CI-only.
- **`lib/wire/fuzz/` — `cargo-fuzz` harnesses for every `dol-wire`
  decoder.** Per `docs/v2_plan.md` §57 (*"Fuzz targets in
  `lib/wire/fuzz/` for every decoder, run in CI nightly"*) and §78.
  Two targets ship in this PR: `unframe` (envelope parser) and
  `decode_program` (full `Decode`-driven `Program` decode). Each
  target is a standalone crate (own `[workspace]`), so it does not
  perturb the parent workspace; cargo-fuzz requires the nightly
  toolchain. The CI `fuzz-smoke` job runs each target for 60 seconds
  on every PR; crashes upload as artifacts. Adding a new target is
  documented in `lib/wire/fuzz/README.md`.
- **`dol_core::diag::ErrorCode` (`u16`, `repr(transparent)`).** Replaces
  the placeholder `Code(&'static str)` with the layered scheme
  `layer * 1000 + serial` documented in `docs/v2_plan.md` §22. The type
  itself is `no_std + no_alloc` (`Copy`, 2 B), and `Display` writes the
  canonical `DOL{NNNN:04}` form straight into a `core::fmt::Formatter`
  without allocating. Helpers: `as_u16`, `layer`, `serial`. The 23
  built-in codes keep their identifiers and serial numbers (`DOL0001`
  → `ErrorCode(1)`, `DOL5001` → `ErrorCode(5001)`, …); only the wire-
  level type changes. Per the v2 "no compatibility shims" rule the old
  `Code` type is deleted, not aliased.
- **`defmt` cargo feature on `dol-core`** (per `docs/v2_plan.md` §32).
  Off by default; opting in adds `defmt::Format` impls on `ErrorCode`,
  `Span`, and `FileId` for embedded-friendly logging on Cortex-M /
  RISC-V targets. ~10× smaller log code than `core::fmt::Display`. New
  workspace dependency `defmt = "0.3"` (vendor-neutral, `no_std`).
- **`dol_core::hash` chokepoint module** (gated by the new `hash` cargo
  feature, off by default to preserve the minimal `no_std + alloc`
  shape). Newtype `Hasher` wrapping `blake3::Hasher`, plus `Digest32`
  (4 B), `Digest128` (16 B), `Digest256` (32 B) byte-aliases and
  one-shot `hash32` / `hash128` / `hash256` helpers. All truncations are
  byte-prefix-consistent. `dol-wire`'s `hash` feature now routes through
  this module; the workspace contains zero direct `blake3::*` calls
  outside `dol_core::hash` (per `docs/v2_plan.md` §25 — "everyone uses
  these, never blake3 directly. Lets us swap if needed").
- **`dol_core::policy::Quota` + `QuotaCaps`.** Cross-traversal cumulative
  resource quota (per-tenant / per-session) layered on top of `Budget`.
  Includes `Quota::consume(&Budget) -> Result<(), BudgetError>`,
  saturating-add accounting, and an `unbounded()` preset.
- **`dol_core::raw` module.** Reserved `unsafe`-audit boundary for
  future POD wire-cast types (`bytemuck` / `zerocopy`). Currently empty;
  ships with a charter doc + `lib/core/src/raw/README.md` review
  checklist. The workspace remains `forbid(unsafe_code)`.
- **`Operation::Raw` placement decision documented.** Stays in `dol-ir`
  behind the `raw` feature; rustdoc on `RawOp` now captures the
  `RAW_PASSTHROUGH` capability-tag invariant and the rationale for not
  splitting into `dol-backends-common`.

#### Changed

- **`.github/workflows/ci.yml` — `cargo-udeps` gate** (per
  `docs/v2_plan.md` §78). Runs on the nightly toolchain
  (`cargo-udeps` requires `rustc -Zunstable-options`), `--workspace
  --all-targets --all-features`, side-by-side with `cargo-deny`.
  Marked `continue-on-error: true` for the duration of the v2 cut so
  nightly rustc instability cannot block the merge queue; the gate
  flips to a hard fail once the workspace shape settles.
- **`dol_expr::Interner` swapped to BLAKE3-truncated content addressing**
  (per `docs/v2_plan.md` §37). `StrId` is now the leading 32 bits of
  `dol_core::hash::hash32(content)` — cryptographic, the same byte
  prefix BLAKE3 emits for content addressing across the workspace. The
  on-disk Symbol IDs of the workspace's compile-time extension dispatch
  (`lib/{pipeline,stream}/src/extension.rs`) keep using FNV-1a, since
  those need `const fn` evaluation and are scoped to a closed,
  vendor-blessed set; user-supplied strings (the interner's domain) are
  the case where collision-resistance against adversarial input
  matters. `dol-expr` unconditionally enables `dol-core/hash`.
- **`.github/workflows/ci.yml`** — the per-crate "Serde round-trip
  gate" is renamed to "Wire round-trip gate" and now runs the
  `dol-wire` integration tests (`program_roundtrip`, `decode_robustness`,
  `decode_core_roundtrip`).
- Workspace docs and per-crate `lib.rs` feature tables updated to
  describe the v2 invariant: `Serialize` is universal, `Deserialize` is
  not.

#### Deferred (separate follow-up PR)

- *(none — all phase-2 items have landed.)* The previous deferred item
  (promotion of `clippy::indexing_slicing` /
  `clippy::arithmetic_side_effects` from `warn` to `deny`) is now done:
  the workspace lint table at `Cargo.toml:74-75` carries `= "deny"`,
  and every production call-site has been audited per the prose at
  `Cargo.toml:68-73`. Out-of-scope follow-ups tracked separately:
  full `lib/expr` cut-over from the variant `ExprNode` enum to a
  `PackedNode`-only representation (touches `arena.rs`, `expr.rs`,
  every `lower_*`); widening `Id<Tag>` (and therefore `StrId`) to 64
  bits to drive the §37 collision surface to ~zero; and concrete
  backend impls of `KvStore` / `Catalog` (will land with the first
  backend).

## [0.2.0] — 2026-05

### v2 cut — coordinated, core-first, no compatibility shims

This release locks in the v2 *shape* of the workspace. The non-negotiable
invariants below are enforced by the workspace lint table and by CI gates
landing alongside this release. There are **no** v1/v2 feature gates, no
`Deserialize` migrators, and no deprecation aliases — old names are deleted
and new ones re-exported in their place.

#### Workspace shape

- **All packages bumped `0.1.0 → 0.2.0`.** Single coordinated cut.
- **Workspace-level lint table** in `Cargo.toml` (`[workspace.lints]`):
  - `rust.unsafe_code = "forbid"` — re-asserts the per-crate forbid so a
    future crate added without the per-crate attribute still fails closed.
    The plan's `lib/core/src/raw/` carve-out for POD wire-cast types is
    intentionally **not** introduced; it will land when the first such type
    is needed.
  - `clippy.{unwrap_used, expect_used, panic} = "deny"` — workspace-wide.
    Every crate root carries
    `#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used,
    clippy::panic, clippy::indexing_slicing,
    clippy::arithmetic_side_effects))]` so the deny applies only to non-test
    compilation. Documented invariant panics (e.g. `Interner::intern`'s
    collision panic, `Symbol::from_hash`'s unreachable arm, the arena
    overflow guards, the `xtask` manifest-dir parent assertion) carry
    per-fn `#[allow]` attributes with prose justifications.
  - `clippy.{indexing_slicing, arithmetic_side_effects} = "warn"` —
    intentionally not denied: they fire on every `+`, `-`, `arr[i]` in
    correct integer arithmetic (calendar conversions, hashing inner loops,
    well-bounded indexing). Converting them to `checked_*` / `.get(...)`
    would rewrite a substantial amount of provably-correct code with no
    runtime benefit. CI surfaces new offenders without breaking the build;
    a function-level audit and the eventual `deny` flip is tracked as a
    follow-up.
- **Every crate** (12 packages) now opts into the workspace lints via
  `[lints] workspace = true` instead of crate-local lint declarations.

#### `dol-core` — `no_std + alloc` is now the default

- **Default features stripped.** `lib/core/Cargo.toml`:
  `default = ["std", "serde", "geo", "network", "datetime", "numeric"]`
  → `default = []`. The crate now produces a `no_std + alloc`-clean library
  out of the box. Embedded targets that previously relied on
  `default-features = false` continue to build identically; server consumers
  must now explicitly request `std`, `serde`, and the typesystem-variant
  families they use.
- **Workspace dep declaration** in the root `Cargo.toml` carries
  `default-features = false`, so any consumer that says
  `dol-core = { workspace = true }` inherits the minimal shape and must
  forward only the features it actually needs through its own feature
  pass-throughs.
- **New `core::signing` module** (`Signer`, `Verifier`, `VerifyError`).
  Trait *shape* only — no algorithm picked, no key-material story baked in.
  Both traits are `no_std + alloc`-friendly; concrete implementations
  (Ed25519, ECDSA-P256, HMAC, HSM-backed) live in downstream crates or in
  application code that wires DOL into a host environment. `VerifyError`
  variants are intentionally coarse (`Mismatch` / `Malformed` /
  `Unavailable`) so verifiers do not leak the shape of the failure.

#### Decisions recorded for v2

- **`Id<Tag>` uses `NonZeroU32`, not `NonMaxU32`.** Reserves `0` rather than
  `u32::MAX` as the sentinel; the niche size is identical
  (`Option<Id<T>> == 4 bytes`) and `serde::Deserialize` already rejects `0`.
  Switching to `NonMaxU32` would buy nothing and would require either a
  third-party crate or a hand-rolled wrapper.
- **No `lib/core/src/raw/` yet.** Defer the auditable `unsafe_code` carve-out
  until the first POD wire-cast type lands.

### Outstanding for v2 (tracked in the v2-cut PR)

These items are explicit in the v2 plan and land in subsequent commits on
the same release line; this entry is updated as each phase merges.

#### Phase 2 — eliminate panic-paths in production code ✅

- Added `#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used,
  clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects))]`
  to every crate root so the workspace lint denies apply only to non-test
  compilation.
- **Removed `dol_schema::Entity::field`** (was `#[deprecated]` since 0.1.0).
  Per the v2 "no deprecation aliases" rule, the panicking convenience is
  gone; callers use `try_field(name)` and handle the `Option` explicitly.
- **`dol_core::datetime::utc_date_parts` / `utc_datetime_parts`** now fall
  back to the Unix epoch on a pre-epoch system clock instead of panicking.
- **`dol_pipeline::PipelinePayload::encode` / `dol_stream::*::encode`** now
  emit a single-byte `0xFF` sentinel — an invalid postcard varint
  discriminant — when serialization fails, so the matching `decode`
  rejects with a clean codec error rather than the encoder panicking.
- **`dol_ir::Program::extend`** is now fallible: returns
  `Result<&mut Self, ExtendError>` (variants `ArenaConflict` /
  `CatalogConflict`) instead of panicking on the documented "id remapping
  not yet implemented" path. `ExtendError` is re-exported alongside
  `Program`.
- Documented invariant panics (interner collision, infallible
  `NonZeroU32` constructions, arena overflow, internal UTF-8 invariants)
  carry per-fn `#[allow(clippy::expect_used | clippy::panic)]` with prose
  justifications.
- The five clippy lints flipped from `warn` to `deny`/`warn` per the
  table above.

#### Phase 3 — `dol-wire::Decoder` and the Deserialize strip

- **Architectural foundation landed in 0.2.0:** new `dol_wire::decoder`
  module with the [`Decode`] trait, a [`Reader`] cursor, and the helpers
  every in-memory IR/AST `Decode` impl will compose on. Highlights:
  - [`Decode`] takes `&mut Budget` so recursive payloads cannot blow the
    call stack with adversarial nesting (every recursive read is wrapped
    in `budget.descend(…)`).
  - [`Reader`] exposes only bounds-checked reads — no `unwrap`, no panic
    on truncated input. Helpers cover `read_u8`, `read_bytes`,
    `read_varint_u32` (5-byte cap), `read_varint_u64` (10-byte cap), and
    `read_seq_bytes` (varint length + payload, budget-charged).
  - [`DecodeError`] enumerates: `Eof { needed, had }`, `InvalidVariant
    { type_name, seen }`, `LengthOverflow`, `Utf8`, `Budget(BudgetError)`,
    `Custom(&'static str)`. `BudgetError → DecodeError` via `From`, so
    `?` chaining works in every impl.
  - `Decode` impls landed for the postcard-byte-compatible primitive set:
    `bool`, `u8`/`i8`, `u16`/`u32`/`u64`/`usize` (varint), `i16`/`i32`/
    `i64`/`isize` (zig-zag varint), `String`, `Option<T>`, `Vec<T>`.
  - Architectural proof: a `DecodeWrap` compound type (label + count +
    flags) demonstrates field-order decoding with budget descent. 11
    unit tests cover round-tripping, EOF, invalid discriminants, varint
    overflow, invalid UTF-8, and budget exhaustion.
  - The wire format is documented as a table in the module-level rustdoc.
- **Cut-over follow-up (separate PR):** the remaining ~150 hand-written
  `Decode` impls across `dol-core`, `dol-expr`, `dol-schema`, `dol-ir`,
  `dol-pipeline`, `dol-stream` plus the corresponding
  `#[derive(serde::Deserialize)]` deletions and the
  `decode_postcard<T: Deserialize>` / `decode_json<T: Deserialize>`
  retirement. Until that PR merges, the legacy helpers remain the
  runtime entry points; the v2 [`Decode`] trait is reachable via
  `Reader::new(...)` + `T::decode(...)` for any type that has been
  migrated. The byte format is fixed in v2 (0.2.0) so the follow-up is
  pure mechanical fill-in, not architecture.

#### Phase 4 — Budget threading & doc-test gate

- **CI no-default-features doctest gate** added to the `test` job:
  `cargo test --workspace --doc --no-default-features`. Catches doc
  examples that quietly depend on `std::*` or on a feature-gated type
  without the appropriate `cfg`. The gate currently passes locally
  against the 0.2.0 cut.
- **`xtask budget-gate` subcommand & CI gate** — scans every
  `lib/**/src/**/*.rs` source file and refuses any `pub fn
  (walk|visit|decode|lower)*` whose signature lacks `&mut Budget`.
  Functions that legitimately do not need a budget (the documented
  unbounded `lower_expr`, the `tree::func::lower` builder helper, the
  legacy serde decoders slated for retirement in the Phase 3 cut-over)
  carry an explicit `// budget-gate: opt-out: <reason>` line attached to
  the offending site. Wired into the `test` CI job. The scanner has its
  own unit tests (5 cases) so it does not silently rot.
- **`dol_expr::lower::lower_exprs` / `lower_filters` / `lower_order_by`
  now take `&mut Budget`.** They previously composed the unbounded
  `lower_expr`; threading a budget makes the four-builder
  `dol_query::*::try_build` paths uniformly bounded. `LowerError` gains
  `From<dol_core::policy::BudgetError>`.
- **`dol_query::*::try_build()` family.** Each of the five builders
  (`DeleteQuery`, `UpdateQuery`, `InsertQuery`, `UpsertQuery`,
  `GetQuery`) now exposes a fallible
  `try_build(self) -> Result<Program, BuildError>` that threads a
  default `Budget::new(Limits::host())` through `lower_*`. The old
  infallible `build()` and its prose-justified
  `#[allow(clippy::expect_used)]` exemptions are deleted. `BuildError`
  is the new public error type, with a variant per failure site
  (`Filter` / `Having` / `Projection` / `GroupBy` / `OrderBy` /
  `SetValue { column, cause }` / `JoinOn`). Test fixtures call
  `.try_build().expect(...)` to keep the assertion shape clear.
- **Outstanding:** the recursive enums (`Value`, `Literal<'a>`,
  `LiteralRange<'a>`, `ValueRange`, `EnumDef`, `DataType`, `StructField`)
  and every `Decode` impl in `dol-expr`, `dol-schema`, `dol-ir`,
  `dol-pipeline`, `dol-stream`, plus the `#[derive(serde::Deserialize)]`
  strip + retirement of `decode_postcard<T: Deserialize>` /
  `decode_json<T: Deserialize>` remain the focused Phase 3 follow-up
  PRs. The byte format, trait shape, and the dol-core leaf-type impls
  are fixed in 0.2.0 — every subsequent PR is incremental within that
  contract.

#### Phase 3 follow-up — dol-core leaf-type `Decode` impls

- **`Decode` impls landed for the dol-core leaf type families.** Covers
  `BitString`, `FileId`, `Date`, `Time`, `DateTime`, `Offset`,
  `TimestampTz`, `Interval`, `Decimal`, `Point`, `Line`, `Segment`,
  `Rect`, `Circle`. Each impl threads `&mut Budget` through every field
  via `budget.descend(...)` so adversarial nested input charges depth
  honestly.
- **Primitive prerequisites** added to the `Decode` core: `f32`, `f64`
  (8-byte little-endian, matching postcard), `u128` / `i128` (varint /
  zig-zag varint up to 19 bytes), `Box<str>`, `Box<[u8]>`, and a fixed
  `read_array::<N>()` helper for postcard's prefix-free `[u8; N]` shape.
- **Domain validation in `Decode`.** `Date::decode` rejects
  month / day out of range; `Time::decode` rejects clock-field overflow;
  `Offset::decode` rejects out-of-range timezone offsets;
  `Decimal::decode` rejects scale > `MAX_SCALE`;
  `BitString::decode` rejects byte counts that disagree with
  `len.div_ceil(8)`. Malformed wire input surfaces as
  `DecodeError::Custom(...)` rather than constructing an invalid value.
- **`decode_core_roundtrip` integration test** asserts byte-for-byte
  parity with `postcard::to_allocvec(&v)` for every covered type. 17
  positive round-trips + 3 negative-path tests (invalid date, invalid
  decimal scale, mismatched bit-string byte count). Gated by the new
  `dol-wire` pass-through features `datetime`, `numeric`, `geo`,
  `network`, all rolled into `full`.
- **Deferred to the next PR:** the recursive enums (`Value`,
  `Literal<'a>`, `DataType`, `EnumDef`, `LiteralRange<'a>`,
  `ValueRange`, `StructField`); the `serde(untagged)` types
  (`IpAddr`, `MacAddr`, `Path`) — postcard currently returns
  `WontImplement` for these, so they need a hand-written
  `Encode`/`Decode` pair with an explicit discriminant byte;
  `Span` / `SpanTable` (need a `Span::from_raw_u64` accessor that
  doesn't exist yet).

### IR redesign — breaking

The IR has been redesigned around a single, universal `Operation` enum. As
the project never shipped a stable predecessor, the changelog no longer
narrates the redesign as a v1→v2 migration; this entry describes the
current shape directly.

- **`dol_ir::Operation` is the IR.** A noun/verb hybrid: structural /
  governance variants are nouns (`Schema`, `Field`, `Index`, `Lookup`,
  `Policy`, `Mask`, `Quota`, `Audit`) carrying a `StructuralVerb`
  (`Create` / `Drop` / `Alter` / `Rename` / `Truncate`); data / query /
  authorization variants are verbs (`Insert`, `Update`, `Replace`,
  `Delete`, `Upsert`, `Append`, `Query`, `Probe`, `Describe`, `Grant`,
  `Revoke`); meta variants are `Tx`, `Extension`, and the feature-gated
  `Raw`. `size_of::<Operation>() == 16` is asserted at compile time and
  reported by `xtask size`.
- **Universal addressing primitives** — `Symbol`, `Locator`, `Target`,
  `TargetKind`, `SchemaBinding` — back every operation.
- **Schema catalog** — `SchemaCatalog`, `CatalogEntry`, `TypeEntry`, plus
  `SchemaRef` / `CatalogId` / `SchemaId` — let programs reference schemas
  as data instead of via embedded Rust type walls.
- **Open capability vocabulary** — `CapabilityTag`, `CapabilitySet`,
  `CapabilityCheck` — alongside the bitset `BackendCapabilities`. Every
  `Operation` reports `kind() -> OpKind` and
  `required_capabilities()`.
- **DML payloads reference arena nodes.** `Insert` carries an
  `InsertSource` (`Node(NodeId)` / `FromQuery(NodeId)` / `Bindings` /
  `FromPath(Symbol)` / `FromExpr(NodeId)`); `Update`, `Delete`, and
  `Upsert` each carry a single `node: NodeId` pointing at the
  corresponding `ExprNode::{Update,Delete,Upsert}` body.
- **Typed `ExtensionPayload` trait** in `dol_ir::operation` plus
  `OperationExtension::{from_payload, decode_as}` for typed extension
  registration. `dol-pipeline` ships a `PipelinePayload` (wrapping
  `Graph` under `dol.pipeline/graph` v1) and `dol-stream` ships
  `WindowPayload`, `TimeSeriesPayload`, and `SamplePayload` (under
  `dol.stream/{window,timeseries,iot.sample}` v1).
- **All `dol-query` builders emit `Operation` directly** (`GetQuery`,
  `InsertQuery`, `UpdateQuery`, `DeleteQuery`, `UpsertQuery`). `.build()`
  returns `dol_ir::Program` with one `Operation` whose body is held in
  the program's `ExprArena`. Helper modules: `ddl::{define_entity,
  drop_entity, define_lookup, drop_lookup, drop_field, rename_field,
  define_index}`, `storage::{put_blob, put_blob_from_path, get_blob,
  list_blobs, read_file, write_file, write_file_from_path, move_file}`,
  `control::{grant, revoke, define_policy, tx_begin, tx_commit,
  tx_rollback, tx_atomic}`.
- **Structured `BackendError`** (`#[non_exhaustive]`) carrying
  `dol_core::Diagnostic` + optional `Span`, with `Capability`,
  `Extension`, and `AclDenied` variants.
- **`dol-check`** operates on `Program::operations`, reporting per-tag
  diagnostics keyed by a structured `CapabilityCheck`.
- **`dol-fmt`** prints the `Operation` form.
- Operation modules are grouped by category under
  `lib/ir/src/operation/{ddl,dml,dql,acl,meta,shared,tx}`. Flat
  re-exports remain at `dol_ir::operation::*`.

- **`dol-core` granular features** — `geo`, `network`, `datetime`, `numeric`
  (all default-on). Disabling any one drops the corresponding `Value` /
  `Literal` / `DataType` / `TypeError` variants and the owning module from
  the compiled crate, suitable for embedded targets that only need a subset
  of the type system. The umbrella `dol` crate exposes pass-through features
  with the same names; the `iot-min` preset now actively omits `geo` and
  `network` while keeping `datetime` and `numeric`.
- CI matrix job `dol-core-features` exercises representative
  `--no-default-features` combinations end-to-end.

### Changed

- **`dol-core` value/type system file split** (internal refactor; no public
  API change):
  - `value/` is now per-concern (`enum_def` / `range` / `classify` /
    `accessors` / `display` / `from_impls`) with tests living in
    `value/tests/<concern>.rs` siblings.
  - `Literal<'a>` lifted to a top-level `lib/core/src/literal/` module
    (`enum_def` / `range` / `constructors` / `display` / `conversions`),
    reflecting that it is an AST node, not a runtime value. Public paths
    (`dol_core::Literal`, `dol_core::LiteralRange`, `dol_core::value::Literal`)
    continue to resolve.
  - `data_type/` split mirrors the same shape (`enum_def` / `struct_field`
    / `classify` / `conformance` / `constructors` + tests).
  - Shared `fmt_uuid` helper lifted to `crate::format`.
- `xtask nostd` promoted from `cargo check` to `cargo test`. Test bodies
  are now type-checked under `--no-default-features` so `alloc`-import
  regressions can no longer hide. Existing `cargo test -p dol-core
  --no-default-features` failures (31 errors) fixed via per-test
  `alloc::string::ToString` / `alloc::vec` imports.
- `network::ParseMacAddrError` now implements `core::error::Error`
  unconditionally (was gated behind `feature = "std"`; the crate's MSRV is
  1.85, well past the 1.81 stabilisation of `core::error::Error`).
- `dol-core/std` now implies `dol-core/datetime`. The wall-clock helpers
  (`datetime::today`/`now`/`now_tz`) live in the datetime module and would
  be orphaned otherwise.
- **Workspace restructure**: crates now live in three top-level buckets —
  `lib/` (libraries), `tools/` (developer tools), and `backends/` (concrete
  `Backend` implementations; reserved, currently empty). Folder names drop
  the `dol-` prefix; published package names retain it. The dependency DAG
  is unchanged in shape but enforced by the bucket invariants.
- **Merged `dol-span` + `dol-diag` + `dol-types` into a single `dol-core`
  crate.** `span` and `diag` remain as namespaced sub-modules; the
  value/type system is the crate's flat root surface (`dol_core::Value`,
  `dol_core::DataType`, …). The most-used items (`Value`, `Literal`,
  `DataType`, `TypeError`, `Span`, `FileId`, `Diagnostic`, `Severity`,
  `Code`) are also re-exported flat at the crate root.
- The umbrella `dol` crate now exposes `dol::core` (was `dol::types`) and
  drops the `arena` / `span` / `diag` features (folded into `core`).

### Removed

- **`dol-arena`** — unused by any internal crate. The arena/interner code
  remains in git history if a future external consumer needs it.

### Added

- `no_std + alloc` support for the language layer: `dol-core` and `dol-expr`
  now compile with `--no-default-features` and on `thumbv7em-none-eabihf`.
  CI's `cross-compile` job and `xtask nostd` enforce both
- New `std` feature on `dol-types` (default-on) gating
  `datetime::today / now / now_tz`; `core::error::Error` impl on `TypeError`
  is unconditional
- New `std` feature on `dol-expr` (default-off) that simply forwards to
  `dol-types/std`
- `#[non_exhaustive]` applied to the public, growable enums in the language
  layer (`Value`, `Literal`, `DataType`, `BinOp`, `UnaryOp`, `Order`,
  `LockHint`, `ConflictClause`, `JoinType`, `ExprNode`, `tree::Expr`,
  `tree::op::UnaryOp`, `OpKind`, `FuncKind`, `FieldStep`, plus `StructField`)
- `#![warn(missing_docs)]` enabled on `dol-types` and `dol-expr` (with a
  module-level allowlist tracking the pending doc backlog)
- Property-based tests for `dol-types` and `dol-expr` invariants (interner
  determinism + canonical JSON form, validated-constructor exhaustiveness,
  `DataType::accepts` consistency, primitive serde round-trip)
- Tree → arena lowering invariant tests for `dol-expr`
- Criterion benches for the interner (hot/unique/serde)
- Publish metadata (`license`, `repository`, `keywords`, `categories`,
  `[package.metadata.docs.rs]`) on `dol-types` and `dol-expr`
- `docs/expr.md` (architecture of the language layer) and
  `docs/STABILITY.md` (semver and wire-format policy)
- Comprehensive unit tests for `dol-expr`, `dol-ir`, and `dol-query`
- `dol-query` crate for backend-neutral query entry points
- `dol-schema` crate for Entity/Field/constraints and DDL builders
- `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md` governance files
- `CHANGELOG.md` for tracking changes
- `justfile` for developer task automation
- `rustfmt.toml` and `clippy.toml` for consistent code style enforcement
- GitHub issue templates
- `#![deny(unsafe_code)]` lint attribute on all crates
- `PartialEq` / `Eq` derives on core IR types for improved testability
- `rust-version = "1.85"` MSRV declaration in workspace `Cargo.toml`

### Changed

- Aligned `rust-toolchain.toml` to version 1.94 (matching CI)
- Updated `.vscode/settings.json` from stale protobuf config to rust-analyzer

### Removed

- Removed backend crates: `dol-sql`, `dol-kv`, `dol-objects`, `dol-spreadsheet`
- Removed auxiliary crates: `dol-migration`, `dol-config`
- Removed deprecated crates: `dol-core`, `dol-builder`
- Removed unused workspace dependencies: `serde_norway`, `toml_crate`, `tokio`
- Stale `crates/` directory (duplicate Cargo.toml stubs with broken `serde_yaml` refs)
- Irrelevant `docker.yml` workflow (referenced non-existent `idp-host`/`edge-oidc` binaries)
- Irrelevant `proto.yml` workflow (referenced non-existent `proto/` directory)
- Unused PostgreSQL and Redis service containers from CI test job

## [0.1.0] — 2025-01-01

### Added

- Initial release of DOL — Data Operating Language
- Expression engine (`dol-expr`) with composable AST
- Schema language (`dol-entity`) with const-compatible Entity/Field/FieldType
- Intermediate representation (`dol-ir`) with Backend trait
- Builder API (`dol-builder`) with method-chain builders
- SQL backend (`dol-sql`) with 7 dialect presets
- Key-value backend (`dol-kv`)
- Object storage backend (`dol-objects`)
- Migration system (`dol-migration`) with schema diff engine
- Unified configuration (`dol-config`) for TOML/YAML/JSON
