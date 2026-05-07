# DOL — Data Operating Language

A type-safe, composable query and schema language for Rust.

DOL provides a backend-agnostic intermediate representation for queries and
schema definitions, along with a fluent builder API for constructing them. The
vocabulary is store-neutral: DOL talks about *entities*, *fields*, *relations*,
*identity*, *invariants*, and *lookups* — never tables, columns, foreign keys,
primary keys, CHECK constraints, or indexes. Backends translate that vocabulary
into whatever shape their store understands.

## Example

```rust
use dol_schema::{DataType, Entity, EntityConstraint, Field, RefAction, RelationRef};

let users = Entity::new("users", vec![
    Field::new("id", DataType::Uuid).identity(),
    Field::new("email", DataType::varying_string(255)).unique().lookup(),
    Field::new("status", DataType::unbounded_string()).default("'active'"),
    Field::new("seq", DataType::Int32).auto_assign(),
]).with_constraints(vec![
    EntityConstraint::unique(["email"]),
]);

let memberships = Entity::new("memberships", vec![
    Field::new("user_id", DataType::Uuid)
        .references_full(RelationRef::new("users", "id").on_delete(RefAction::Cascade)),
    Field::new("org_id", DataType::Uuid),
]).with_constraints(vec![
    EntityConstraint::identity(["user_id", "org_id"]),
]);
```

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

DOL is structured as a three-tier workspace with a strict, acyclic dependency
DAG. Folder names drop the `dol-` prefix; **published package names keep it**.

```
lib/        ── reusable libraries (depend only on each other)
tools/      ── developer tools that consume the libraries
backends/   ── concrete `dol_ir::Backend` implementations
xtask/      ── workspace task runner (not published)
```

| Folder · Crate                                   | Description                                                                  |
| ------------------------------------------------ | ---------------------------------------------------------------------------- |
| [`lib/core`](lib/core/README.md) · `dol-core`        | Source spans, structured diagnostics, and the value/type system (`Value`, `Literal`, `DataType`, `Decimal`, …) |
| [`lib/expr`](lib/expr/README.md) · `dol-expr`        | Composable expression AST (16 B packed `ExprNode`, arena, interner)          |
| [`lib/schema`](lib/schema/README.md) · `dol-schema`  | Entities, fields, constraints, relations, lookups, policies                  |
| [`lib/ir`](lib/ir/README.md) · `dol-ir`              | Canonical IR: `Statement`, `Program`, `Backend` trait, `BackendCapabilities` |
| [`lib/wire`](lib/wire/README.md) · `dol-wire`        | Canonical wire envelope, postcard / JSON codec helpers, BLAKE3 content hash  |
| [`lib/query`](lib/query/README.md) · `dol-query`     | Fluent builder DSL → produces `dol-ir::Statement`; also hosts the streaming / pipeline / IoT IR (formerly `dol-stream` and `dol-pipeline`) |
| [`lib/dol`](lib/dol/README.md) · `dol`               | Umbrella facade with `core`, `full`, `iot-min` presets                       |
| [`tools/check`](tools/check/README.md) · `dol-check` | Static validator (type / schema / capability / lint passes)                  |
| [`tools/fmt`](tools/fmt/README.md) · `dol-fmt`       | Canonical pretty-printer for IR programs                                     |
| [`backends/`](backends/README.md)                    | Reserved for `dol-backend-<store>` crates (currently empty)                  |
| [`xtask`](xtask/README.md)                           | Workspace task runner (size report, `no_std` check, doc build, README check) |

```
lib/core ─┬─► lib/expr ────┐
          ├─► lib/schema ──┴─► lib/ir ─┬─► lib/wire
          └────────────────┘           ├─► lib/query  (absorbs former
                                       │              lib/stream + lib/pipeline)
                                       ├─► tools/check
                                       ├─► tools/fmt
                                       └─► backends/<store>
                                              lib/dol  (umbrella)
```

Invariants enforced by the workspace structure:

- Nothing under `lib/` may depend on `tools/` or `backends/`.
- `tools/*` may depend on `lib/*` but not on each other or `backends/`.
- `backends/*` depend only on `lib/ir` (+ optionally `lib/wire`).
- No crate depends on `lib/dol`; the umbrella is leaf consumer surface.

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

## Per-crate documentation

Every crate ships its own `README.md` next to its `Cargo.toml` (linked in the
table above). The `xtask readme` subcommand verifies that each workspace
member has a non-empty README and declares it in `Cargo.toml`:

```bash
cargo run -p xtask -- readme
```

## Design references

| Topic                         | Document                                          |
| ----------------------------- | ------------------------------------------------- |
| Stability policy              | [`docs/STABILITY.md`](docs/STABILITY.md)          |
| `dol-ir` reference            | [`docs/IR.md`](docs/IR.md)                        |
| `dol-ir` design RFC           | [`docs/rfcs/0001-ir.md`](docs/rfcs/0001-ir.md)    |
| Expression / arena layer      | [`docs/expr.md`](docs/expr.md)                    |
| Release notes                 | [`CHANGELOG.md`](CHANGELOG.md)                    |

## License

BSD 3-Clause License. See [LICENSE](LICENSE) for details.
