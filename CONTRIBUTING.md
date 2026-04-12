# Contributing to DOL

Thank you for your interest in contributing to DOL! This guide will help you
get started.

## Getting Started

1. **Fork** the repository and clone your fork.
2. Create a **feature branch** from `main`:
   ```bash
   git checkout -b feat/my-feature
   ```
3. Make your changes, ensuring they follow the project conventions.
4. Run the full CI checks locally (see below).
5. Open a **Pull Request** against `main`.

## Development Setup

**Requirements:**

- Rust 1.85+ (edition 2024) — install via [rustup](https://rustup.rs/)
- `cargo-deny` for license/audit checks

```bash
# Install the project toolchain (uses rust-toolchain.toml)
rustup show

# Run all checks (equivalent to CI)
just check   # or manually:
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo deny check
```

## Code Style

- **Formatting:** Enforced by `rustfmt` — run `cargo fmt --all` before committing.
- **Linting:** Clippy with `-D warnings` — fix all warnings.
- **Documentation:** All public items should have doc comments (`///`).
  Crates enforce `#![warn(missing_docs)]`.
- **Unsafe code:** Forbidden — crates enforce `#![deny(unsafe_code)]`.

## Testing

- Write unit tests alongside the code they test (in `#[cfg(test)] mod tests`).
- Write doc tests for public API examples.
- Run `cargo test --workspace --all-features` to execute the full suite.
- Feature-gated code should be tested under the relevant feature flag.

## Commit Messages

Use conventional commit messages:

```
feat: add CTE support to GetBuilder
fix: handle empty field list in InsertBuilder
docs: improve Model API documentation
test: add edge case tests for expression rendering
refactor: simplify dialect type resolution
```

## Pull Request Process

1. Ensure CI passes (formatting, linting, tests, security audit).
2. Add or update tests for any changed functionality.
3. Update documentation if public APIs change.
4. Request review from a maintainer.

## Crate Architecture

```
dol-core/
  ├── dol-expr     — Expression AST (leaf, no deps)
  ├── dol-model    — Model/Field/FieldType (leaf, no deps)
  ├── dol-ir       — IR + Backend trait (depends on expr, model)
  └── dol-builder  — Builder API (depends on expr, model, ir)
dol-sql            — SQL rendering + dialect system
dol-kv             — Key-value backend
dol-objects        — Object storage backend
dol-migration      — Migration system
dol-config         — Unified configuration
dol                — Umbrella crate (re-exports everything)
```

When adding a new feature, place it in the lowest appropriate crate to minimize
coupling.

## Reporting Issues

- Use the **Bug Report** template for bugs.
- Use the **Feature Request** template for enhancements.
- Include minimal reproduction steps where possible.

## License

By contributing, you agree that your contributions will be licensed under the
same [BSD 3-Clause License](LICENSE) that covers the project.
