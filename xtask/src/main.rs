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
//! | `nostd`  | run `cargo check --no-default-features` on `no_std` crates. |
//! | `doc`    | build workspace docs with all features.                  |
//! | `readme` | verify every workspace member has a non-empty README.md. |
//! | `help`   | print this list.                                         |
//!
//! See `justfile` for higher-level recipes that wrap these.

use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| "help".into());
    let rest: Vec<String> = args.collect();

    let ok = match cmd.as_str() {
        "size" => size_report(),
        "nostd" => nostd_check(),
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
        "size_of::<dol_types::Value>()                = {}",
        size_of::<dol_types::Value>()
    );
    println!(
        "size_of::<dol_types::Literal<'static>>()     = {}",
        size_of::<dol_types::Literal<'static>>()
    );
    println!(
        "size_of::<dol_expr::ExprNode>()              = {}",
        size_of::<dol_expr::ExprNode>()
    );
    println!(
        "size_of::<dol_ir::Statement>()               = {}",
        size_of::<dol_ir::Statement>()
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
    budget!(dol_types::Value, 24);
    budget!(dol_types::Literal<'static>, 32);
    budget!(dol_expr::ExprNode, 32);
    // Boxing the heavy DML / DDL / storage variants brings `Statement`
    // comfortably under the 64-byte budget set by the implementation plan.
    budget!(dol_ir::Statement, 64);
    ok
}

/// Verify the leaf no_std crates still build without `std`.
fn nostd_check() -> bool {
    let crates = ["dol-arena", "dol-span", "dol-diag", "dol-types", "dol-expr"];
    for c in crates {
        let ok = run_cargo(&["check", "-p", c, "--no-default-features"], &[]);
        if !ok {
            eprintln!("xtask: nostd check failed for {c}");
            return false;
        }
    }
    true
}

/// Verify every workspace member has a non-empty `README.md` and that its
/// `Cargo.toml` declares it via `readme = "README.md"`. Keeps per-crate docs
/// from silently rotting away.
fn readme_check() -> bool {
    use std::fs;
    use std::path::PathBuf;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // `xtask` lives at `<workspace>/xtask`, so the workspace root is the parent.
    let workspace_root = manifest_dir
        .parent()
        .expect("xtask manifest dir has a parent")
        .to_path_buf();

    // Discover every workspace member by scanning `crates/*` plus the `xtask`
    // crate itself. Keeps this self-contained (no `cargo metadata` parsing).
    let mut members: Vec<PathBuf> = Vec::new();
    let crates_dir = workspace_root.join("crates");
    match fs::read_dir(&crates_dir) {
        Ok(entries) => {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() && p.join("Cargo.toml").is_file() {
                    members.push(p);
                }
            }
        }
        Err(e) => {
            eprintln!("xtask: cannot read {}: {e}", crates_dir.display());
            return false;
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
