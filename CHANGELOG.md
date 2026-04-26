# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
