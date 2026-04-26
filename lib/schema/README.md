# `dol-schema`

DOL schema language. The universal vocabulary for describing structured data
shapes — store-neutral by design.

In DOL, an **Entity** is the neutral term for any structured data shape:

- SQL → table
- Document store → collection
- Object store → bucket schema
- File system → typed resource

A **Field** is a named property within an Entity. A **DataType** is the
backend-agnostic logical type descriptor (re-exported from `dol-core`).
**Constraints**, **relations**, **lookups**, and **policies** describe the
non-shape semantics.

## Features

| feature | default | effect                              |
| ------- | :-----: | ----------------------------------- |
| `serde` |   ✔     | `Serialize` / `Deserialize` derives |

`serde` is on by default because schema definitions are almost always
serialized (config files, wire envelopes, golden tests). Embedded users can
opt out with `default-features = false`.

## Example

```rust
use dol_schema::{DataType, Entity, EntityConstraint, Field, RefAction, RelationRef};

let users = Entity::new("users", vec![
    Field::new("id", DataType::Uuid).identity(),
    Field::new("email", DataType::varying_string(255)).unique().lookup(),
    Field::new("status", DataType::unbounded_string()).default("'active'"),
]).with_constraints(vec![
    EntityConstraint::unique(["email"]),
]);

let memberships = Entity::new("memberships", vec![
    Field::new("user_id", DataType::Uuid)
        .references_full(RelationRef::new("users", "id").on_delete(RefAction::Cascade)),
    Field::new("org_id", DataType::Uuid),
]);
```

See the rustdoc for the full API.
