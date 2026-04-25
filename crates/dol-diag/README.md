# `dol-diag`

Structured diagnostics. Every DOL crate that validates user input
(`dol-check`, `dol-wire`, `dol-schema`, …) emits `Diagnostic` values rather
than panicking.

A diagnostic is:

- a stable `Code` (so external tooling can pin behaviour),
- a `Severity` (`Error`, `Warning`, `Lint`, `Note`),
- a primary `dol_span::Span`,
- a short human message,
- zero or more secondary `Label`s, `Note`s, and `FixIt` hints.

The catalogue of built-in codes lives in [`code`](src/code.rs). Downstream
crates may mint their own codes by passing a `&'static str`.

## Features

| feature  | default | effect                                                |
| -------- | :-----: | ----------------------------------------------------- |
| `serde`  |         | `Serialize` derives across the diagnostic types       |
| `std`    |         | std-only error/IO machinery                           |
| `render` |         | terminal pretty-printer (pulls Unicode width tables)  |

This crate is `no_std + alloc` by default.

## Example

```rust,ignore
use dol_diag::{Code, Diagnostic, Severity};
use dol_span::Span;

let d = Diagnostic::new(Code::TYPE_MISMATCH, Severity::Error, Span::default(),
    "expected Int32, found String");
```

See the rustdoc for the full API.
