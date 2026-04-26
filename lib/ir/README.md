# `dol-ir`

The canonical intermediate representation for all DOL backends. Contains:

- **`Statement`** — the closed enum of every DOL operation (DML, DDL,
  control, transaction, storage, extension). Heavy variants are boxed, so
  `size_of::<Statement>() == 24` (≤ 64 B budget).
- **`Program`** — a `Statement` paired with the `ExprArena` and `Interner`
  needed to resolve any arena references it carries.
- **`Transaction`** — transaction-scope envelope used by control statements.
- **`Backend`** trait + `BackendCapabilities` + `BackendError`.

Schema constraint types (`RefAction`, `ComputedKind`, `RelationRef`,
`EntityConstraint`) are re-exported from `dol-schema` for convenience.

## Construction ergonomics

Every boxed variant has a `From<T> for Statement` impl, so callers don't
write `Box::new(...)` themselves:

```rust,ignore
let stmt: dol_ir::Statement = my_query_node.into();
```

## Static guarantees

- `size_of::<Statement>() ≤ 64` — enforced by `xtask size`.
- `Statement: Send + Sync + 'static` — enforced by static assertion.

## Features

| feature | default | effect                                                                   |
| ------- | :-----: | ------------------------------------------------------------------------ |
| `serde` |         | `Serialize` / `Deserialize` for `Statement`, `Program`, `Transaction`, … |

Enabling `serde` turns on `dol-expr/serde` and `smallvec/serde` transitively.

## Example

```rust,ignore
use dol_ir::{Program, Statement};
// `Program` is what every `Backend::compile` consumes.
fn run<B: dol_ir::Backend>(b: &B, p: &Program) -> Result<B::Output, _> {
    b.compile(p)
}
```

See the rustdoc for the full API.
