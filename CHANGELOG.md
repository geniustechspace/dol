# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- **Outstanding:** the ~150 `Decode` impls on `dol-core`, `dol-expr`,
  `dol-schema`, `dol-ir`, `dol-pipeline`, `dol-stream` IR/AST types and
  the corresponding `#[derive(serde::Deserialize)]` strip + retirement
  of `decode_postcard<T: Deserialize>` / `decode_json<T: Deserialize>`
  remain the focused Phase 3 cut-over PR. The byte format, trait shape,
  and primitive impls are fixed in 0.2.0.

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
