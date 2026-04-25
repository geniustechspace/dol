# DOL — Developer Task Automation
# Install: cargo install just
# Usage:  just <recipe>

# Default: run all checks (equivalent to CI)
default: check

# Run all CI checks
check: fmt clippy test deny

# Format check
fmt:
    cargo fmt --all -- --check

# Format fix
fmt-fix:
    cargo fmt --all

# Lint with Clippy
clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run all tests
test:
    cargo test --workspace --all-features

# Run tests for a specific crate
test-crate crate:
    cargo test -p {{crate}}

# Quick check (no tests)
quick:
    cargo check --workspace --all-features

# Security audit
deny:
    cargo deny check

# Build documentation
doc:
    cargo doc --workspace --all-features --no-deps --open

# Print the in-memory size of every size-budgeted public IR type.
size:
    cargo run -q -p xtask -- size

# Verify the leaf no_std crates compile without `std`.
nostd:
    cargo run -q -p xtask -- nostd

# Build workspace docs without opening them.
docs:
    cargo run -q -p xtask -- doc

# Verify every workspace member ships a non-empty README.md.
readme:
    cargo run -q -p xtask -- readme

# Run all xtask gates that CI runs (size + nostd + readme).
xtask-gates: size nostd readme

# Cross-compile the no_std leaves for `thumbv7em-none-eabihf`. Requires
# `rustup target add thumbv7em-none-eabihf` once.
cross-thumbv7em:
    cargo check -p dol-arena -p dol-span -p dol-diag \
        --no-default-features --target thumbv7em-none-eabihf

# Compile-check the umbrella's `iot-min` preset.
iot-min:
    cargo check -p dol --no-default-features --features iot-min

# Reproduce CI locally, in the order CI runs.
ci: fmt clippy test xtask-gates iot-min deny

# Clean build artifacts
clean:
    cargo clean
