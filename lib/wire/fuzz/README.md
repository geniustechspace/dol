# `dol-wire-fuzz`

Fuzz harnesses for the `dol-wire` decoders, per `docs/v2_plan.md` §57:
*"Fuzz targets in `lib/wire/fuzz/` for every decoder, run in CI nightly."*

## Targets

| target            | covers                                                                |
| ----------------- | --------------------------------------------------------------------- |
| `unframe`         | `dol_wire::unframe` — header / envelope parsing                       |
| `decode_program`  | `dol_wire::program::decode` — full `Decode`-driven `Program` decode  |

## Running locally

```bash
cargo install cargo-fuzz                  # one-time
cd lib/wire/fuzz
cargo +nightly fuzz run unframe        -- -max_total_time=60
cargo +nightly fuzz run decode_program -- -max_total_time=60
```

## CI

The `fuzz-smoke` job in `.github/workflows/ci.yml` runs each target for
60 seconds on the nightly toolchain. Crash artifacts (under
`artifacts/`) are uploaded for triage.

## Adding a new target

1. Add a `fuzz_targets/<name>.rs` with a `fuzz_target!(|data: &[u8]| { … })`.
2. Add a `[[bin]] name = "<name>"` entry in `Cargo.toml`.
3. Add a `cargo +nightly fuzz run <name> -- -max_total_time=60` step to
   the `fuzz-smoke` CI job.
4. Document the target's contract (what bytes mean, what's verified)
   in the file header.
