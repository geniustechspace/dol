# `dol-query`

Backend-neutral fluent builder DSL for DOL. Produces `dol_ir::Statement`
values that any `dol_ir::Backend` can compile.

Unlike a low-level builder that requires a static `&Entity` reference,
`dol-query` accepts **both** `Entity` references and plain entity-name
strings. This makes it suitable for dynamic / runtime scenarios — REST
APIs, configuration-driven pipelines — where the entity name is only known
at runtime and no static schema definition exists.

## Verbs

`Query::from(...)` followed by:

- `.get()` — read
- `.insert()` — DML insert
- `.update()` — DML update
- `.delete()` — DML delete
- `.upsert()` — DML upsert (with conflict resolution)

Each verb returns a builder that exposes the relevant predicates,
projections, joins, and limits, and terminates with `.build() -> Program`.

## Features

| feature | default | effect                                                         |
| ------- | :-----: | -------------------------------------------------------------- |
| `serde` |         | forwards `serde` to `dol-ir`, `dol-expr`, `dol-schema`, `dol-types` |
| `sql`   |         | reserved for future SQL-dialect-aware helpers                  |

## Example

```rust
use dol_query::Query;
use dol_schema::{Entity, Field, DataType};
use dol_expr::tree::{field, param};

let users = Entity::new("users", vec![
    Field::new("id", DataType::Uuid).identity(),
    Field::new("email", DataType::unbounded_string()),
]);

let dol_ir::Program { stmt, interner, .. } = Query::from(&users)
    .get()
    .filter(field("id").eq(param()))
    .build();
```

See the rustdoc for the full API.
