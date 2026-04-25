# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `no_std + alloc` support for the language layer: `dol-types` and `dol-expr`
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
