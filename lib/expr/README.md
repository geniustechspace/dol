# `dol-expr`

Expression engine. Provides two layers:

- **`tree::Expr<'a>`** — a recursive tree AST with a fluent builder API.
  This is what users compose expressions with (`field("x").eq(param())`).
- **`ExprNode`** — a flat, 16-byte `bytemuck::Pod` node packed as
  `op + flags + aux + a + b + c`. Tree expressions are lowered into
  this packed form (in an `ExprArena`) before rendering. Inspect via
  the typed `ExprNode::as_*` accessors and construct via the typed
  `ExprNode::*` / `ExprArena::alloc_*` helpers — never via raw
  `a`/`b`/`c`.

Also exports the supporting machinery: `ExprArena`, `Interner`,
`BuildSession`, all the typed ids (`NodeId`, `FieldId`, `LiteralId`,
`ArrayLitId`, …), the opcode discriminator (`ExprOp`), and the
canonical side-pool node structs (`QueryNode`, `InsertNode`,
`UpdateNode`, `DeleteNode`, `UpsertNode`, `JoinNode`, `WindowNode`,
`CaseNode`, `ArrayLitNode`, …).

## Size guarantee (asserted in `xtask size`)

```text
size_of::<ExprNode>() == 16
```

## Features

| feature | default | effect                                                            |
| ------- | :-----: | ----------------------------------------------------------------- |
| `serde` |         | `Serialize` for every AST/arena type and `Interner`. v2 wire-in goes through `dol-wire::Decode`. |

`Interner` has a hand-rolled deterministic encoder that round-trips
through its canonical sequence-of-strings form, so JSON output is
stable regardless of the in-memory storage.

## Example

```rust,ignore
use dol_expr::tree::{field, param};

let e = field("user.id").eq(param());
```

See the rustdoc for the full API.
