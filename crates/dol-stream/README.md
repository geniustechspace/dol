# `dol-stream`

Streaming / time-series / IoT IR. Extends the core IR with operators that
don't fit cleanly into batch SQL semantics:

- **Windows** — tumbling, hopping, session, count-based.
- **Watermarks** — with allowed lateness and triggers.
- **Time-series ops** — `time_bucket`, `downsample`, `gap_fill`, `locf`,
  `rate`, `delta`.
- **Telemetry / IoT vocabulary** — sensors, actuators, samples, retention
  policies, store-and-forward intent, QoS hints, payload codecs.

These types are pure data; they hook into `dol_ir::Statement` via the
`Statement::Extension` seam and are validated by `dol-check`.

## Features

| feature | default | effect                                                                   |
| ------- | :-----: | ------------------------------------------------------------------------ |
| `serde` |         | `Serialize` / `Deserialize` (forwards to `dol-ir`, `dol-expr`, `smallvec`) |

## Example

```rust,ignore
use dol_stream::{WindowSpec, Watermark};

let w = WindowSpec::tumbling(std::time::Duration::from_secs(60));
let wm = Watermark::default();
```

See the rustdoc for the full API.
