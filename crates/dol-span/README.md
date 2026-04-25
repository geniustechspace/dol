# `dol-span`

Compact source spans for diagnostics. A `Span` is a `(FileId, start, length)`
triple packed into 8 bytes (`u16 + u24 + u24`). Spans are kept out of the hot
AST/IR path; they live in a side `SpanTable` keyed by the AST node's id.

## Encoding

`Span` is a `repr(transparent)` `u64`:

| bits     | field    |
| -------- | -------- |
| `0..16`  | `file`   |
| `16..40` | `start`  |
| `40..64` | `length` |

`length` is capped at 16 MiB which is enough for any source line and most
files.

## Features

| feature | default | effect                              |
| ------- | :-----: | ----------------------------------- |
| `serde` |         | `Serialize` / `Deserialize` derives |
| `std`   |         | enables std-only error impls        |

This crate is `no_std + alloc` and has no DOL dependencies.

## Example

```rust,ignore
use dol_span::{FileId, Span, SpanTable};

let s = Span::new(FileId(0), 12, 4);
assert_eq!(s.file(), FileId(0));
```

See the rustdoc for the full API.
