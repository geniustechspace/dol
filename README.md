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

| Crate                                   | Description                                              |
| --------------------------------------- | -------------------------------------------------------- |
| [`dol-types`](libs/dol-types)           | Leaf: `Value`, `Literal`, `DataType`                     |
| [`dol-expr`](libs/dol-expr)             | Composable expression AST (ExprNode ≤ 32B, arena, interner) |
| [`dol-ir`](libs/dol-ir)                 | Canonical IR: `Statement` enum and `Backend` trait       |
| [`dol-schema`](libs/dol-schema)         | Schema: `Entity`, `Field`, constraints, definition builders |
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
