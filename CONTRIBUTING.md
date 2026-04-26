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
- **Unsafe code:** Do not introduce `unsafe` code — crates enforce `#![deny(unsafe_code)]`.

## Testing

- Write unit tests alongside the code they test (in `#[cfg(test)] mod tests`).
- Write doc tests for public API examples.
- Run `cargo test --workspace --all-features` to execute the full suite.
- Feature-gated code should be tested under the relevant feature flag.

## Serde Representation

The wire format produced by `serde::Serialize` is part of the public contract
of every type in `dol-core`. Pick the enum representation deliberately:

- **Default (externally tagged)** — use for any enum whose variants would be
  ambiguous on the wire: variants that share a primitive shape (e.g. several
  number-like or string-like variants), variants whose payloads have
  overlapping field sets, or unit variants that need to be distinguishable.
  This is the safe default and is what `Value`, `Literal`, `DataType`, and
  the geo enums use.
- **`#[serde(untagged)]`** — only when every pair of variants is
  _unambiguous_ on the wire (distinct primitive type, distinct fixed-width
  array length, or disjoint required field sets). Good fits are
  "newtype-style multiplexers" of fixed-width payloads. `IpAddr`
  (`[u8; 4]` vs `[u8; 16]`) and `MacAddr` (`[u8; 6]` vs `[u8; 8]`) qualify.
- **`#[serde(tag = "kind")]` (internally tagged)** — prefer over `untagged`
  when you want a cleaner wire format than the default but the variants are
  not structurally unambiguous.

When introducing or changing the serde representation of a public type, add
a `# Serde representation` doc section to the type and a wire-shape test
under `lib/core/tests/serde_roundtrip.rs` so the format is asserted,
not just inferred.

## Commit Messages

Use conventional commit messages:

```markdown
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

```markdown
dol-core → dol-expr → dol-ir
↑
dol-schema → dol-query
```

| Crate        | Role                                                            |
| ------------ | --------------------------------------------------------------- |
| `dol-core`   | Leaf: `Value`, `Literal`, `DataType`                            |
| `dol-expr`   | Composable expression AST — operators, functions, windows       |
| `dol-ir`     | Intermediate representation — `Statement` enum, `Backend` trait |
| `dol-schema` | Schema language — `Entity`, `Field`, constraints, DDL           |
| `dol-query`  | Query entry point + builders → produce IR                       |

When adding a new feature, place it in the lowest appropriate crate to minimize
coupling.

## Reporting Issues

- Use the **Bug Report** template for bugs.
- Use the **Feature Request** template for enhancements.
- Include minimal reproduction steps where possible.

## License

By contributing, you agree that your contributions will be licensed under the
same [BSD 3-Clause License](LICENSE) that covers the project.
