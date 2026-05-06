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

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )
)]

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
        "budget-gate" => budget_gate(),
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
           size         print size_of for size-budgeted public IR types\n  \
           nostd        verify the no_std layer compiles without `std`\n  \
           mcu          `cargo check` the no_std layer against MCU targets\n             \
                        (default: riscv32imac-unknown-none-elf,\n             \
                         thumbv7em-none-eabihf; pass `--target=<triple>` to override)\n  \
           doc          build workspace documentation\n  \
           readme       verify every workspace member has a non-empty README.md\n  \
           budget-gate  v2 invariant: every recursive entry point\n             \
                        (`pub fn (walk|visit|decode|lower)*`) must take a\n             \
                        `&mut Budget`. Fails CI on any offender that lacks\n             \
                        an explicit `// budget-gate: opt-out` marker.\n  \
           help         show this message\n"
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

// ---------------------------------------------------------------------------
// budget-gate
// ---------------------------------------------------------------------------

/// v2 invariant gate: every public recursive entry point in the workspace —
/// any `pub fn (walk|visit|decode|lower)\w*\(...)` — must accept a
/// `&mut Budget` so adversarial input cannot overflow the call stack or
/// host memory.
///
/// The gate scans every `lib/**/src/**/*.rs` source file. For each
/// matching `pub fn`, the function header (signature up to the opening
/// `{`) must contain the literal token `Budget`. Functions that
/// legitimately do not need a budget (e.g. plain builder helpers, the
/// pre-Phase-3 serde decoders that will be retired, the unbounded
/// `lower_expr` convenience) carry an explicit
/// `// budget-gate: opt-out: <reason>` line within the three lines
/// preceding the `pub fn` token. Any unmarked offender is a CI failure.
///
/// This intentionally lives in `xtask` rather than as a `clippy` lint or
/// build script: it's an architectural rule, not a syntax rule, and the
/// allowlist is explicit prose attached to each opt-out site rather than
/// a global config file.
fn budget_gate() -> bool {
    use std::fs;
    use std::path::PathBuf;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Lint exemption: see `readme_check` — `xtask` is exempt from the
    // workspace no-panic invariant per the v2 plan. The `expect`
    // documents that `CARGO_MANIFEST_DIR` always has a parent during
    // `cargo run`.
    #[allow(clippy::expect_used)]
    let workspace_root = manifest_dir
        .parent()
        .expect("xtask manifest dir has a parent")
        .to_path_buf();

    let mut sources: Vec<PathBuf> = Vec::new();
    collect_rs(&workspace_root.join("lib"), &mut sources);

    // Order matters only for stable output.
    sources.sort();

    let mut offenders: Vec<String> = Vec::new();
    for path in &sources {
        let body = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("xtask: cannot read {}: {e}", path.display());
                return false;
            }
        };
        scan_file(path, &body, &mut offenders);
    }

    if offenders.is_empty() {
        println!(
            "xtask budget-gate: all recursive entry points in {} files thread `&mut Budget`",
            sources.len()
        );
        true
    } else {
        eprintln!(
            "xtask budget-gate: {} recursive entry point(s) lack `&mut Budget` and have no `// budget-gate: opt-out` marker:",
            offenders.len()
        );
        for o in &offenders {
            eprintln!("  {o}");
        }
        eprintln!(
            "\nFix: thread a `&mut dol_core::policy::Budget` through the function, \
             or annotate it with `// budget-gate: opt-out: <reason>` on the line(s) \
             immediately preceding `pub fn`."
        );
        false
    }
}

fn collect_rs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if p.is_dir() {
            // Skip target/ caches and tests/ trees — gate is about
            // production library code only. Use `to_string_lossy()`
            // rather than `to_str()` so a non-UTF-8 directory name
            // (legal on Linux/macOS) still lets us match the filter
            // strings instead of silently skipping the directory.
            let name = p
                .file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default();
            if matches!(name.as_ref(), "target" | "tests" | "benches" | "examples") {
                continue;
            }
            collect_rs(&p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            // Also skip `tests.rs` and `*_tests.rs` modules — production
            // gate, not a test-code gate.
            let name = p
                .file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default();
            if name == "tests.rs" || name.ends_with("_tests.rs") {
                continue;
            }
            out.push(p);
        }
    }
}

fn scan_file(path: &std::path::Path, body: &str, offenders: &mut Vec<String>) {
    // Pattern: `pub fn (walk|visit|decode|lower)<name>(... )` possibly
    // spanning multiple lines. We work line-by-line, greedily consuming
    // continuation lines until we find the matching `)` that closes the
    // parameter list.
    let lines: Vec<&str> = body.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(name) = match_recursive_entry(line) {
            // Collect the full signature (up to `{` or `;`).
            let mut header = String::new();
            let mut j = i;
            while j < lines.len() {
                header.push_str(lines[j]);
                header.push('\n');
                if lines[j].contains('{') || lines[j].trim_end().ends_with(';') {
                    break;
                }
                j += 1;
            }

            let has_budget = header.contains("Budget");
            // Scan backward across any contiguous block of comments,
            // blank lines, and `#[...]` attributes immediately preceding
            // the `pub fn` for an explicit opt-out marker. This avoids
            // false positives when the function carries multiple
            // attributes or a doc comment block between the marker and
            // the signature.
            let mut has_optout = false;
            let mut k = i;
            while k > 0 {
                k -= 1;
                let prev = lines[k].trim_start();
                if prev.is_empty()
                    || prev.starts_with("//")
                    || prev.starts_with("#[")
                    || prev.starts_with("#![")
                {
                    if prev.contains("budget-gate: opt-out") {
                        has_optout = true;
                        break;
                    }
                    continue;
                }
                // Hit a line that's neither comment / blank / attribute —
                // we've left the function's leading block.
                break;
            }

            if !has_budget && !has_optout {
                offenders.push(format!(
                    "{}:{}: pub fn {name} (no `&mut Budget`, no opt-out marker)",
                    path.display(),
                    i + 1,
                ));
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
}

/// Return the function name when `line` contains a `pub fn (walk|visit|decode|lower)<name>(`.
fn match_recursive_entry(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let after = trimmed.strip_prefix("pub fn ")?;
    // Take everything up to the first `(` or `<` (generic params) or whitespace.
    let end = after
        .find(|c: char| c == '(' || c == '<' || c.is_whitespace())
        .unwrap_or(after.len());
    let name = &after[..end];
    if name.starts_with("walk")
        || name.starts_with("visit")
        || name.starts_with("decode")
        || name.starts_with("lower")
    {
        Some(name)
    } else {
        None
    }
}

#[cfg(test)]
mod budget_gate_tests {
    use super::*;

    #[test]
    fn matches_pub_fn_walk_visit_decode_lower() {
        assert_eq!(
            match_recursive_entry("pub fn walk_node(...)"),
            Some("walk_node")
        );
        assert_eq!(
            match_recursive_entry("    pub fn visit_op<T>(...)"),
            Some("visit_op")
        );
        assert_eq!(match_recursive_entry("pub fn decode_x()"), Some("decode_x"));
        assert_eq!(
            match_recursive_entry("pub fn lower_expr(...)"),
            Some("lower_expr")
        );
    }

    #[test]
    fn ignores_unrelated_pub_fns() {
        assert_eq!(match_recursive_entry("pub fn build()"), None);
        assert_eq!(match_recursive_entry("pub fn encode()"), None);
        assert_eq!(match_recursive_entry("fn lower_priv()"), None); // not pub
        // Substring-match on `lower` is intentional — `lower<'a>` is the
        // `LOWER(expr)` SQL helper that opts out explicitly via marker.
    }

    #[test]
    fn flags_signature_without_budget() {
        let src = "\
pub fn walk_op(x: u32) -> u32 {
    x
}
";
        let mut offenders: Vec<String> = Vec::new();
        scan_file(std::path::Path::new("test.rs"), src, &mut offenders);
        assert_eq!(offenders.len(), 1);
        assert!(offenders[0].contains("walk_op"));
    }

    #[test]
    fn accepts_signature_with_budget() {
        let src = "\
pub fn walk_op(x: u32, b: &mut Budget) -> u32 {
    x
}
";
        let mut offenders: Vec<String> = Vec::new();
        scan_file(std::path::Path::new("test.rs"), src, &mut offenders);
        assert!(offenders.is_empty());
    }

    #[test]
    fn accepts_explicit_opt_out_across_attributes() {
        let src = "\
/// Doc.
// budget-gate: opt-out: documented unbounded entry point.
#[cfg(feature = \"json\")]
pub fn decode_thing(s: &str) -> u32 {
    0
}
";
        let mut offenders: Vec<String> = Vec::new();
        scan_file(std::path::Path::new("test.rs"), src, &mut offenders);
        assert!(offenders.is_empty(), "offenders: {offenders:?}");
    }

    #[test]
    fn handles_multiline_signatures() {
        let src = "\
pub fn lower_expr(
    expr: &Expr,
    arena: &mut Arena,
) -> Result<(), Err> {
    Ok(())
}
";
        let mut offenders: Vec<String> = Vec::new();
        scan_file(std::path::Path::new("test.rs"), src, &mut offenders);
        assert_eq!(offenders.len(), 1);
        assert!(offenders[0].contains("lower_expr"));
    }
}
