# `dol`

Umbrella facade crate. Re-exports the individual `dol-*` crates behind cargo
feature flags so downstream consumers can pick the slice of the stack they
need without pulling in the full dependency tree.

## Layered surface

| Feature      | Pulls in       | Purpose                                                    |
| ------------ | -------------- | ---------------------------------------------------------- |
| *(default)*  | `dol-types`    | Leaf primitives: `Value`, `Literal`, `DataType`, …         |
| `arena`      | `dol-arena`    | Generic typed arena + interner.                            |
| `span`       | `dol-span`     | Compact source spans for diagnostics.                      |
| `diag`       | `dol-diag`     | Diagnostic model + code catalogue (implies `span`).        |
| `expr`       | `dol-expr`     | Expression arena + tree DSL.                               |
| `schema`     | `dol-schema`   | Entities, fields, constraints, relations, lookups.         |
| `ir`         | `dol-ir`       | `Statement`, `Program`, `Backend`, `BackendCapabilities`.  |
| `pipeline`   | `dol-pipeline` | Source → Transform → Sink dataflow IR.                     |
| `stream`     | `dol-stream`   | Windows, watermarks, time-series, IoT vocabulary.          |
| `wire`       | `dol-wire`     | Canonical wire envelope + postcard / JSON codec helpers.   |
| `check`      | `dol-check`    | Static validator (type / schema / capability / lint).      |
| `fmt`        | `dol-fmt`      | Canonical pretty-printer.                                  |
| `query`      | `dol-query`    | Fluent builder DSL.                                        |

## Curated presets

| Preset    | Layers                                        | Use case                                        |
| --------- | --------------------------------------------- | ----------------------------------------------- |
| `core`    | `expr + schema + ir + query`                  | "Full programs" build, no codecs / streaming    |
| `full`    | every layer DOL ships                         | Library / tooling consumers                     |
| `iot-min` | `expr + schema + ir + stream + wire/postcard` | Minimal IoT-edge slice (fits `thumbv7em` budget) |

## Universal `serde`

Adding the `serde` feature turns on `serde` on every active sub-crate via
`?` activations, so it is a no-op on layers you have not enabled.

Note: the leaf crates `dol-types` and `dol-schema` already enable `serde` by
default, because the cost is negligible and they're almost always used with
serialization. Disable them with `default-features = false` on each crate
individually if you need the no-op build.

## Example

```toml
# Cargo.toml
[dependencies]
dol = { version = "0.1", features = ["core", "wire", "serde"] }
```

```rust,ignore
use dol::query::Query;
let program = Query::from("users").get().build();
```

See the rustdoc and the per-crate READMEs for the full surface.
