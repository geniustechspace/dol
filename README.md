# DOL — Data Operating Language

A type-safe, composable query and schema language for Rust.

DOL provides a backend-agnostic intermediate representation for queries and
schema definitions, along with a fluent builder API for constructing them.

## Architecture

```markdown
Builders → IR
(human API) (neutral AST)
───────────────────────────────
Query::from("users").get()    Statement::Query
Query::from("users").insert() Statement::Insert
Query::from("users").upsert() Statement::Upsert
Entity::define("users")       Statement::DefineEntity
...                            ...
```

**Builders** provide a fluent, method-chain API for constructing operations.

**IR** is a backend-agnostic intermediate representation that captures the
intent of every operation without being tied to any specific storage engine.

## Workspace Structure

| Crate                                   | Description                                              |
| --------------------------------------- | -------------------------------------------------------- |
| [`dol-types`](libs/dol-types)           | Leaf: `Value`, `Literal`, `DataType`                     |
| [`dol-expr`](libs/dol-expr)             | Composable expression AST (ExprNode ≤ 32B, arena, interner) |
| [`dol-ir`](libs/dol-ir)                 | Canonical IR: `Statement` enum and `Backend` trait       |
| [`dol-schema`](libs/dol-schema)         | Schema: `Entity`, `Field`, constraints, DDL builders     |
| [`dol-query`](libs/dol-query)           | Query entry point + builders → produce `dol-ir::Statement` |

```
dol-types  →  dol-expr  →  dol-ir
                              ↑
                          dol-schema  →  dol-query
```

## Building

```bash
# Check the entire workspace
cargo check --workspace

# Run all tests
cargo test --workspace

# Run tests with all features
cargo test --workspace --all-features

# Format
cargo fmt --all

# Lint
cargo clippy --workspace --all-features
```

**Requirements:** Rust 1.85+ (edition 2024)

## License

BSD 3-Clause License. See [LICENSE](LICENSE) for details.
