# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
