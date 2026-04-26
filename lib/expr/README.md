# `dol-expr`

Expression engine. Provides two expression representations:

- **`tree::Expr<'a>`** — a recursive tree AST with a fluent builder API.
  This is what users compose expressions with (`field("x").eq(param())`).
- **`ExprNode`** — a flat, arena-based node (≤ 32 bytes) for efficient
  storage and backend processing. Tree expressions are lowered into this
  form before rendering.

Also exports the supporting machinery: `ExprArena`, `Interner`,
`BuildSession`, all the typed ids (`NodeId`, `FieldId`, `LiteralId`, …), and
the canonical AST node structs (`QueryNode`, `InsertNode`, `UpdateNode`,
`DeleteNode`, `UpsertNode`, `JoinNode`, `WindowNode`, `CaseNode`, …).

## Size guarantee (asserted in `xtask size`)

```text
size_of::<ExprNode>() == 32
```

## Features

| feature | default | effect                                                              |
| ------- | :-----: | ------------------------------------------------------------------- |
| `serde` |         | `Serialize` / `Deserialize` for every AST/arena type and `Interner` |

`Interner` has a hand-rolled deterministic codec that round-trips through its
canonical `Vec<Arc<str>>` form, so postcard / JSON output is stable.

## Example

```rust,ignore
use dol_expr::tree::{field, param};

let e = field("user.id").eq(param());
```

See the rustdoc for the full API.
