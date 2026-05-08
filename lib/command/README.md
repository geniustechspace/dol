# `dol-command`

The canonical command-language IR for all DOL backends (formerly
`dol-ir`). Contains:

- **`Operation`** — the closed enum of every DOL operation (DML, DDL,
  control, transaction, storage, extension). Heavy variants are boxed, so
  `size_of::<Operation>() == 24` (≤ 64 B budget).
- **`Program`** — an `Operation` sequence paired with the `ExprArena` and
  `Interner` needed to resolve any arena references it carries.
- **`Backend`** trait + `BackendCapabilities` + `BackendError`.
- **`builders`** — top-down helpers (`define_entity`, `grant`,
  `tx_begin`, `read_file`, …) that emit ready-made `Program`s for the
  verbs that don't need the fluent query DSL.

Schema addressing primitives (`SchemaRef`, `SchemaCatalog`, `TypeBody`,
`EntityConstraint`, `RefAction`, `ComputedKind`, `RelationRef`) live in
`dol-schema` and are imported from there directly — there are no
convenience re-exports at the `dol-command` crate root.

## Construction ergonomics

Every boxed variant has a `From<T> for Operation` impl, so callers don't
write `Box::new(...)` themselves:

```rust,ignore
let op: dol_command::operation::Operation = my_insert.into();
```

## Static guarantees

- `size_of::<Operation>() ≤ 64` — enforced by `xtask size`.
- `Operation: Send + Sync + 'static` — enforced by static assertion.

## Features

| feature | default | effect                                                                    |
| ------- | :-----: | ------------------------------------------------------------------------- |
| `serde` |         | `Serialize` for `Operation`, `Program`, etc. (no `Deserialize` — wire-in uses `dol-wire::Decode`) |

Enabling `serde` turns on `dol-expr/serde` and `smallvec/serde` transitively.

## Example

```rust,ignore
use dol_command::program::Program;
use dol_command::program_ref::ProgramRef;
use dol_command::backend::Backend;

// `ProgramRef` is what every `Backend::compile` consumes.
fn run<B: Backend>(b: &B, p: &Program) -> Result<B::Output, B::Error> {
    b.compile(p.as_ref())
}
```

See the rustdoc for the full API.
