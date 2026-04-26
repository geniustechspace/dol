# `dol-core`

The foundation crate for the DOL stack. Bundles three independently coherent
pieces that every higher `dol-*` crate builds on:

- **Source spans** (`span` namespace) — compact `(file, start, length)`
  triples used by every diagnostic and AST node table.
- **Structured diagnostics** (`diag` namespace) — code-driven error reports
  with labels, notes, and fix-its. Validators emit `Diagnostic`s rather than
  panicking.
- **The DOL value/type system** — the universal logical type vocabulary:
  `Value`, `Literal`, `DataType`, plus the validated primitives (`Decimal`,
  `Date`, `Time`, `Interval`, `IpAddr`, `MacAddr`, `BitString`, `geo::Point`,
  …) and `TypeError`.

This crate has **no DOL dependencies**; every higher layer pulls type
definitions from here so there is exactly one definition of every type in the
workspace.

## Public surface

The most-used items are re-exported flat at the crate root:

```rust
use dol_core::{Value, Literal, DataType, TypeError, Span, FileId,
               Diagnostic, Severity, Code};
```

Sub-namespaces remain accessible for less-frequently used items:

```rust
use dol_core::span::SpanTable;
use dol_core::diag::{Label, Note, FixIt};
use dol_core::datetime::{Date, Time, DateTime, Interval, Offset, TimestampTz};
use dol_core::geo::{Point, Line, Polygon, Rect, Circle, Path, Segment};
use dol_core::network::{IpAddr, MacAddr};
use dol_core::numeric::Decimal;
use dol_core::binary::BitString;
```

## Size guarantees (64-bit targets, asserted in tests)

```text
size_of::<Value>()            == 24
size_of::<Literal<'static>>() == 32
size_of::<Span>()             ==  8
```

## Features

| feature    | default | effect                                                                                     |
| ---------- | :-----: | ------------------------------------------------------------------------------------------ |
| `std`      |    ✔    | Enables wall-clock factory helpers (`datetime::today`/`now`/`now_tz`); implies `datetime`. |
| `serde`    |    ✔    | `Serialize` / `Deserialize` for every public type, including `Span` / `Diagnostic`.        |
| `geo`      |    ✔    | The `geo` module + `Value`/`Literal`/`DataType`/`TypeError` variants for geometric types.  |
| `network`  |    ✔    | The `network` module + `Inet`/`MacAddr` variants on `Value`/`Literal`/`DataType`.          |
| `datetime` |    ✔    | The `datetime` module + `Date`/`Time`/`DateTime`/`TimestampTz`/`Interval` variants.        |
| `numeric`  |    ✔    | The `numeric` module + `Decimal` variants and decimal-related `TypeError` arms.            |

`core::error::Error` is always implemented for `TypeError` and
`network::ParseMacAddrError` (stable since Rust 1.81).

Embedded users can opt out with `default-features = false` for a
`no_std + alloc` build. Granular features can be re-enabled à la carte;
turning any one off drops the corresponding `Value` / `Literal` /
`DataType` / `TypeError` variants and the entire owning module from the
compiled crate.

## Example

```rust
use dol_core::{DataType, Value};

let dt = DataType::varying_string(255);
assert!(dt.accepts(&Value::String("hello".into())).is_ok());
```

See the rustdoc for the full API.
