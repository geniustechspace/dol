# `dol-types`

Leaf primitive types for the DOL stack. No DOL semantics — just the canonical
`DataType` / `Value` / `Literal` triple plus their structural sub-types
(`Decimal`, `Date`, `Time`, `Interval`, `IpAddr`, `MacAddr`, `BitString`,
`geo::Point`, …).

This crate has **no DOL dependencies** and is re-exported by every higher
layer, so there is exactly one definition of every type in the workspace.

## Layers

| Layer       | Types                              | Purpose                                          |
| ----------- | ---------------------------------- | ------------------------------------------------ |
| Values      | `Value`, `Literal`                 | Carry actual data at runtime / in ASTs           |
| Descriptors | `DataType`, `StructField`          | Describe the expected shape of a position        |
| Primitives  | `Decimal`, `Date`, `geo::Point`, … | Structural sub-types with validated constructors |
| Errors      | `TypeError`                        | All validation and conformance errors            |

The bridge between them is `DataType::accepts(&Value) -> Result<(), TypeError>`.

## Size guarantees (64-bit targets, asserted in tests)

```text
size_of::<Value>()            == 24
size_of::<Literal<'static>>() == 32
```

## Features

| feature | default | effect                              |
| ------- | :-----: | ----------------------------------- |
| `serde` |    ✔    | `Serialize` / `Deserialize` derives |

`serde` is on by default because this is a leaf primitive crate and the cost
is negligible. Embedded users can opt out with `default-features = false`.

## Example

```rust
use dol_types::{DataType, Value};

let dt = DataType::varying_string(255);
assert!(dt.accepts(&Value::String("hello".into())).is_ok());
```

See the rustdoc for the full API.
