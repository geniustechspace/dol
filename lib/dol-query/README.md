# `dol-query`

Backend-neutral fluent builder DSL for DOL. Produces query builder data
that can be lowered to `dol_command::program::Program`.

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
projections, joins, and limits.

## Features

| feature | default | effect                                                         |
| ------- | :-----: | -------------------------------------------------------------- |
| `serde` |         | forwards `serde` to `dol-expr`, `dol-schema`, `dol-core` |
| `sql`   |         | reserved for future SQL-dialect-aware helpers                  |

## Example

```rust
use dol_query::Query;
use dol_schema::{Entity, Field, DataType};
use dol_expr::tree::{field, param};
use dol_command::lower_query::BuildProgram;

let users = Entity::new("users", vec![
    Field::new("id", DataType::Uuid).identity(),
    Field::new("email", DataType::unbounded_string()),
]);

let program = Query::from(&users)
    .get()
    .filter(field("id").eq(param()))
    .try_build()
    .expect("example lowers");
```

See the rustdoc for the full API.
