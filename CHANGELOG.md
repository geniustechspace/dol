# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Comprehensive unit tests for `dol-expr`, `dol-entity`, `dol-ir`, and `dol-builder`
- `Render` trait for SQL rendering (replaces `TryToSql`/`ToSql`)
- `TransactionRender` trait for transaction control rendering
- `DefineTypeBuilder`, `DropTypeBuilder`, `DefinePolicyBuilder` for DDL/RLS
- `CompoundSelectBuilder` for UNION/INTERSECT/EXCEPT set operations
- `GetBuilderSqlExt` for compound query and subquery methods on `GetBuilder`
- `dol-query` crate for backend-neutral query entry points
- `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md` governance files
- `CHANGELOG.md` for tracking changes
- `justfile` for developer task automation
- `rustfmt.toml` and `clippy.toml` for consistent code style enforcement
- GitHub issue templates
- Feature matrix CI testing (default, config, migration, full)
- `#![deny(unsafe_code)]` lint attribute on all crates
- `PartialEq` / `Eq` derives on core IR types for improved testability
- `rust-version = "1.85"` MSRV declaration in workspace `Cargo.toml`

### Changed

- **BREAKING**: Removed `all_fields()` — all entity fields are now included by default; use `.columns()` to narrow
- **BREAKING**: Replaced `TryToSql`/`ToSql` traits with unified `Render` trait (`.render()`)
- **BREAKING**: Replaced `TransactionSqlExt` with `TransactionRender` trait
- **BREAKING**: Removed `col()` and `qualified_col()` aliases — use `field()` and `qualified()`
- **BREAKING**: Removed `ModelDefineExt` alias — use `EntityDefineExt`
- **BREAKING**: Removed `add_column()` from `AlterEntityBuilder` — use `add_field()`
- **BREAKING**: Removed `schema()` from `DefineEntityBuilder` — use `namespace()`
- Renamed `dol-model` crate to `dol-entity` (Entity-centric naming)
- Aligned `rust-toolchain.toml` to version 1.94 (matching CI)
- Updated `.vscode/settings.json` from stale protobuf config to rust-analyzer

### Removed

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
