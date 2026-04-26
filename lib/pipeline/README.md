# `dol-pipeline`

Declarative dataflow IR. A pipeline is a typed DAG of nodes joined by typed
edges:

- **`Source`** — feeds rows into the graph
- **`Transform`** — rewrites rows. Any relational/scalar op expressible in
  `dol-expr`, plus higher-order ops like `unnest`, `pivot`, `unpivot`,
  `gap_fill`, `asof_join`, `tdigest`, `approx_*`.
- **`Sink`** — consumes rows out of the graph.

Pipelines are **descriptive only** — this crate provides type inference over
their node schemas; execution is a backend concern.

Pipelines compose with the core IR via the `dol_ir::Statement::Extension`
seam: a `Pipeline` is wrapped in a `StatementExtension` and shipped through
the same wire envelope as any other statement.

## Features

| feature | default | effect                                                                     |
| ------- | :-----: | -------------------------------------------------------------------------- |
| `serde` |         | `Serialize` / `Deserialize` (forwards to `dol-ir`, `dol-expr`, `smallvec`) |

## Example

See the rustdoc for full constructor usage. A typical shape:

```rust,ignore
use dol_pipeline::{Pipeline, Source, Sink};
let pipe = Pipeline::new()
    .source(Source::Entity("readings"))
    .sink(Sink::Entity("aggregates"));
```

See the rustdoc for the full API.
