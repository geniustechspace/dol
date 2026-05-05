//! `xtask` — DOL workspace task runner.
//!
//! Run via:
//!
//! ```text
//! cargo run -p xtask -- <subcommand>
//! ```
//!
//! Subcommands:
//!
//! | command  | purpose                                                  |
//! |----------|----------------------------------------------------------|
//! | `size`   | print `size_of` for the public size-budgeted IR types.   |
//! | `nostd`  | run `cargo test --no-default-features` on `no_std` crates. |
//! | `mcu`    | `cargo check` the `no_std` leaves against bare-metal MCU targets. |
//! | `doc`    | build workspace docs with all features.                  |
//! | `readme` | verify every workspace member has a non-empty README.md. |
//! | `help`   | print this list.                                         |
//!
//! See `justfile` for higher-level recipes that wrap these.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects))]

use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| "help".into());
    let rest: Vec<String> = args.collect();

    let ok = match cmd.as_str() {
        "size" => size_report(),
        "nostd" => nostd_check(),
        "mcu" => mcu_check(&rest),
        "doc" => run_cargo(
            &["doc", "--workspace", "--all-features", "--no-deps"],
            &rest,
        ),
        "readme" => readme_check(),
        "help" | "-h" | "--help" => {
            print_help();
            true
        }
        other => {
            eprintln!("xtask: unknown command `{other}`");
            print_help();
            false
        }
    };

    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn print_help() {
    println!(
        "xtask <subcommand>\n\
         \n\
         Subcommands:\n  \
           size    print size_of for size-budgeted public IR types\n  \
           nostd   verify the no_std layer compiles without `std`\n  \
           mcu     `cargo check` the no_std layer against MCU targets\n             \
                  (default: riscv32imac-unknown-none-elf,\n             \
                   thumbv7em-none-eabihf; pass `--target=<triple>` to override)\n  \
           doc     build workspace documentation\n  \
           readme  verify every workspace member has a non-empty README.md\n  \
           help    show this message\n"
    );
}

fn run_cargo(base: &[&str], extra: &[String]) -> bool {
    let status = Command::new(env!("CARGO")).args(base).args(extra).status();
    match status {
        Ok(s) => s.success(),
        Err(e) => {
            eprintln!("xtask: failed to spawn cargo: {e}");
            false
        }
    }
}

/// Print the in-memory size of every public, size-budgeted DOL type. Failing
/// any of the asserted budgets is a regression that CI must reject.
fn size_report() -> bool {
    use std::mem::size_of;
    println!("--- DOL size report ---");
    println!(
        "size_of::<dol_core::Value>()                = {}",
        size_of::<dol_core::Value>()
    );
    println!(
        "size_of::<dol_core::Literal<'static>>()     = {}",
        size_of::<dol_core::Literal<'static>>()
    );
    println!(
        "size_of::<dol_expr::ExprNode>()              = {}",
        size_of::<dol_expr::ExprNode>()
    );
    println!(
        "size_of::<dol_ir::Operation>()               = {}",
        size_of::<dol_ir::Operation>()
    );

    let mut ok = true;
    macro_rules! budget {
        ($t:ty, $bytes:expr) => {{
            let s = std::mem::size_of::<$t>();
            if s > $bytes {
                eprintln!(
                    "BUDGET VIOLATION: {} = {} bytes (max {})",
                    stringify!($t),
                    s,
                    $bytes
                );
                ok = false;
            }
        }};
    }
    budget!(dol_core::Value, 24);
    budget!(dol_core::Literal<'static>, 32);
    budget!(dol_expr::ExprNode, 32);
    // Boxing every heavy payload (DDL bodies, DML arena handles, governance
    // structs) keeps `Operation` comfortably under its 64-byte budget.
    budget!(dol_ir::Operation, 64);
    ok
}

/// Verify the leaf no_std crates still build *and pass tests* without `std`.
///
/// Promoted from `cargo check` to `cargo test` because `check` does not
/// type-check `#[cfg(test)]` bodies; tests can silently rot when imports
/// from `alloc` are missing under `--no-default-features`.
fn nostd_check() -> bool {
    // dol-core and dol-expr must build *and* pass tests under
    // `--no-default-features`. dol-ir must build under
    // `--no-default-features` (it has no dev-deps that work without std,
    // so we settle for `cargo check`).
    let test_crates = ["dol-core", "dol-expr"];
    for c in test_crates {
        let ok = run_cargo(&["test", "-p", c, "--no-default-features"], &[]);
        if !ok {
            eprintln!("xtask: nostd check failed for {c}");
            return false;
        }
    }
    let check_crates = ["dol-ir", "dol-schema"];
    for c in check_crates {
        let ok = run_cargo(&["check", "-p", c, "--no-default-features"], &[]);
        if !ok {
            eprintln!("xtask: nostd check failed for {c}");
            return false;
        }
    }
    true
}

/// Cross-build the `no_std + alloc`-clean leaves against bare-metal MCU
/// targets via `cargo check --no-default-features --target=<triple>`.
///
/// Targets default to the `thumbv7em-none-eabihf` (Cortex-M4F) and
/// `riscv32imac-unknown-none-elf` (RISC-V 32-bit IMAC) triples that mirror
/// the project's published embedded support matrix. Override with
/// `xtask mcu --target=<triple>` to add or replace entries.
///
/// The target toolchain must already be installed (`rustup target add
/// <triple>`); this command does not install it for you, so callers can
/// fail loudly when the host is missing prerequisites.
fn mcu_check(extra: &[String]) -> bool {
    // Built-in targets that mirror the embedded matrix in `lib/dol`'s
    // `iot-min` feature. Override / extend via `--target=<triple>` flags.
    let mut targets: Vec<String> = vec![
        "riscv32imac-unknown-none-elf".into(),
        "thumbv7em-none-eabihf".into(),
    ];
    let mut overrides: Vec<String> = Vec::new();
    for arg in extra {
        if let Some(t) = arg.strip_prefix("--target=") {
            overrides.push(t.into());
        } else {
            eprintln!("xtask: mcu: unknown argument `{arg}`");
            return false;
        }
    }
    if !overrides.is_empty() {
        targets = overrides;
    }

    // The set of crates known to be `no_std + alloc`-clean. Matches the
    // crates flagged with `#![cfg_attr(not(feature = "std"), no_std)]` and
    // exercised by `nostd_check` plus `dol-schema`.
    let crates = ["dol-core", "dol-expr", "dol-ir", "dol-schema"];
    for target in &targets {
        let mut args: Vec<&str> = Vec::with_capacity(2 * crates.len() + 4);
        args.push("check");
        for c in crates {
            args.push("-p");
            args.push(c);
        }
        args.push("--no-default-features");
        args.push("--target");
        args.push(target);
        let ok = run_cargo(&args, &[]);
        if !ok {
            eprintln!("xtask: mcu check failed for target {target}");
            return false;
        }
    }
    true
}

/// `Cargo.toml` declares it via `readme = "README.md"`. Keeps per-crate docs
/// from silently rotting away.
//
// Lint exemption: `xtask` is an internal build tool and the v2 plan's
// "no_std + alloc default" / no-panic invariants explicitly exempt
// `tools/*` and `xtask`. The `expect` documents an environmental
// invariant — `CARGO_MANIFEST_DIR` always points at a path with a
// parent during `cargo run`.
#[allow(clippy::expect_used)]
fn readme_check() -> bool {
    use std::fs;
    use std::path::PathBuf;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // `xtask` lives at `<workspace>/xtask`, so the workspace root is the parent.
    let workspace_root = manifest_dir
        .parent()
        .expect("xtask manifest dir has a parent")
        .to_path_buf();

    // Discover every workspace member by scanning the three top-level
    // buckets (`lib/*`, `tools/*`, `backends/*`) plus the `xtask` crate
    // itself. Keeps this self-contained (no `cargo metadata` parsing).
    let mut members: Vec<PathBuf> = Vec::new();
    for bucket in ["lib", "tools", "backends"] {
        let bucket_dir = workspace_root.join(bucket);
        match fs::read_dir(&bucket_dir) {
            Ok(entries) => {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.is_dir() && p.join("Cargo.toml").is_file() {
                        members.push(p);
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // `backends/` may legitimately be empty; skip silently.
                continue;
            }
            Err(e) => {
                eprintln!("xtask: cannot read {}: {e}", bucket_dir.display());
                return false;
            }
        }
    }
    members.push(workspace_root.join("xtask"));
    members.sort();

    let mut ok = true;
    for member in &members {
        let name = member.file_name().and_then(|s| s.to_str()).unwrap_or("?");
        let readme = member.join("README.md");
        let manifest = member.join("Cargo.toml");

        match fs::metadata(&readme) {
            Ok(m) if m.len() == 0 => {
                eprintln!("xtask: README.md is empty in `{name}`");
                ok = false;
            }
            Ok(_) => {}
            Err(_) => {
                eprintln!("xtask: missing README.md in `{name}`");
                ok = false;
            }
        }

        match fs::read_to_string(&manifest) {
            Ok(s) => {
                if !s.lines().any(|l| {
                    let t = l.trim();
                    t == "readme = \"README.md\""
                }) {
                    eprintln!("xtask: `{name}/Cargo.toml` is missing `readme = \"README.md\"`");
                    ok = false;
                }
            }
            Err(e) => {
                eprintln!("xtask: cannot read {}: {e}", manifest.display());
                ok = false;
            }
        }
    }

    if ok {
        println!(
            "xtask: README.md present and declared in all {} members",
            members.len()
        );
    }
    ok
}
