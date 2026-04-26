# `dol-wire`

Canonical codecs and content hashing for DOL payloads.

Defines the stable wire envelope: an 8-byte header (`WireHeader`) followed
by a body. The body's encoding is selected per call (postcard for IoT, JSON
for debugging) and is opaque to the header layer. Every payload is prefixed
with a `WireHeader` containing a stable `WireSchemaVersion`. Additive
changes bump `minor`; renames or removals bump `major` and require a
migration shim here.

Content hashing (BLAKE3 over the canonical body bytes) gives every payload
a stable identifier reusable as a migration key, plan-cache key, or IoT
idempotency token.

## Typed `Program` codecs

With the `program` feature (auto-enabled by `postcard` and `json`), this
crate exposes typed helpers in [`mod@program`] that target
`dol_ir::Program` directly:

- `program::encode_postcard` / `program::decode_postcard`
- `program::encode_json` / `program::decode_json`
- `program::content_hash` — canonical BLAKE3 of the postcard body.

The lower-level, payload-agnostic helpers in `mod@postcard` and `mod@json`
remain available for callers that want to encode their own `Serialize`
payloads behind the same envelope.

## Features

| feature    | default | effect                                                       |
| ---------- | :-----: | ------------------------------------------------------------ |
| `serde`    |         | universal serde plumbing on payload types                    |
| `postcard` |         | compact binary codec (ideal for IoT / on-device); pulls `serde` + `std` |
| `json`     |         | human-readable codec (debugging / interop); pulls `serde` + `std`       |
| `hash`     |         | BLAKE3 content hashing                                       |
| `std`      |         | standard library                                             |

Pick exactly one codec for production traffic; both can be enabled side-by-side
in tests.

## Example

```rust,ignore
use dol_wire::program::{encode_postcard, decode_postcard, content_hash};
let bytes = encode_postcard(&program)?;
let hash  = content_hash(&program)?;
let back  = decode_postcard(&bytes)?;
```

See the rustdoc for the full API.
