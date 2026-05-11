//! `xtask` — DOL workspace task runner (M0 scaffold).
//!
//! Implements the five subcommands listed in `dol-rewrite-plan-v2.md` §5.2:
//!
//! | command         | purpose                                                                       |
//! |-----------------|-------------------------------------------------------------------------------|
//! | `budget-gate`   | v2 invariant: every `pub fn (walk|visit|decode|lower|content_hash)_*` in     |
//! |                 | `lib/` must take `&mut Budget` or carry `// budget-gate: opt-out: <reason>`. |
//! | `size-check`    | Asserts size budgets on the v2 IR types. **Stub during M0** (no IR yet).    |
//! | `dag-check`     | Asserts the workspace dependency edges match §4. **Stub during M0**.        |
//! | `no-std-check`  | Builds `dol-core` / `dol-cas` / `dol-ir` for `thumbv7em-none-eabihf`.        |
//! |                 | **Stub during M0** — invoked by CI directly via `cargo check --target`.     |
//! | `size-report`   | Per-crate `.rlib` sizes and IoT demo binary size. **Stub during M0**.       |
//!
//! The four stubs all exit 0 and will be filled in as the crates they
//! inspect gain content during M1–M6.

// xtask is a build-time task runner, not production library code.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| "help".into());

    let ok = match cmd.as_str() {
        "budget-gate" => budget_gate(),
        "size-check" => size_check_stub(),
        "dag-check" => dag_check_stub(),
        "no-std-check" => no_std_check_stub(),
        "size-report" => size_report_stub(),
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
         Subcommands (per dol-rewrite-plan-v2.md §5.2):\n  \
           budget-gate     scan `lib/` for `pub fn (walk|visit|decode|lower|content_hash)_*`\n             \
                           that lack `&mut Budget` and have no `// budget-gate: opt-out` marker.\n  \
           size-check      assert v2 IR size budgets (stub during M0).\n  \
           dag-check       assert workspace dependency DAG (stub during M0).\n  \
           no-std-check    build dol-core/dol-cas/dol-ir for thumbv7em-none-eabihf (stub during M0).\n  \
           size-report     per-crate .rlib and IoT binary sizes (stub during M0).\n  \
           help            show this message\n"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// `budget-gate` — real implementation (file-scan, no IR types needed).
// ──────────────────────────────────────────────────────────────────────────

/// v2 invariant: every public recursive entry point in `lib/` whose name
/// starts with `walk`, `visit`, `decode`, `lower`, or `content_hash` must
/// take `&mut Budget` (or carry `// budget-gate: opt-out: <reason>`).
fn budget_gate() -> bool {
    use std::fs;
    use std::path::PathBuf;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    #[allow(clippy::expect_used)]
    let workspace_root = manifest_dir
        .parent()
        .expect("xtask manifest dir has a parent")
        .to_path_buf();

    let mut sources: Vec<PathBuf> = Vec::new();
    collect_rs(&workspace_root.join("lib"), &mut sources);
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
            let name = p
                .file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default();
            // Skip target/, tests/, benches/, examples/, and the
            // `_legacy_*` folders preserved during the v2 transition.
            if matches!(name.as_ref(), "target" | "tests" | "benches" | "examples")
                || name.starts_with("_legacy")
            {
                continue;
            }
            collect_rs(&p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
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
    let lines: Vec<&str> = body.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(name) = match_recursive_entry(line) {
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

/// Returns the function name when `line` contains a
/// `pub fn (walk|visit|decode|lower|content_hash)_<name>(` signature.
fn match_recursive_entry(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let after = trimmed.strip_prefix("pub fn ")?;
    let end = after
        .find(|c: char| c == '(' || c == '<' || c.is_whitespace())
        .unwrap_or(after.len());
    let name = &after[..end];
    if name.starts_with("walk")
        || name.starts_with("visit")
        || name.starts_with("decode")
        || name.starts_with("lower")
        || name.starts_with("content_hash")
    {
        Some(name)
    } else {
        None
    }
}

// ──────────────────────────────────────────────────────────────────────────
// M0 stubs — exit 0; real impls land as the gated crates gain content.
// ──────────────────────────────────────────────────────────────────────────

fn size_check_stub() -> bool {
    println!(
        "xtask size-check: M0 stub (no IR types defined yet). \
         Real impl lands in M3 alongside `ExprNode` and `Option<Lid<_>>`."
    );
    true
}

fn dag_check_stub() -> bool {
    println!(
        "xtask dag-check: M0 stub (workspace shape declared but not yet \
         programmatically verified). Real impl lands when more than the \
         scaffold crates have content."
    );
    true
}

fn no_std_check_stub() -> bool {
    println!(
        "xtask no-std-check: M0 stub. CI invokes `cargo check --target \
         thumbv7em-none-eabihf` directly on dol-core/dol-cas/dol-ir; the \
         xtask wrapper will subsume that during M1."
    );
    true
}

fn size_report_stub() -> bool {
    println!(
        "xtask size-report: M0 stub. Per-crate .rlib sizes and IoT demo \
         binary size will be reported once M6 lands the IoT preset."
    );
    true
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
        assert_eq!(
            match_recursive_entry("pub fn content_hash_node(...)"),
            Some("content_hash_node")
        );
    }

    #[test]
    fn ignores_unrelated_pub_fns() {
        assert_eq!(match_recursive_entry("pub fn build()"), None);
        assert_eq!(match_recursive_entry("pub fn encode()"), None);
        assert_eq!(match_recursive_entry("fn lower_priv()"), None);
    }

    #[test]
    fn flags_signature_without_budget() {
        let src = "pub fn walk_op(x: u32) -> u32 { x }\n";
        let mut offenders: Vec<String> = Vec::new();
        scan_file(std::path::Path::new("test.rs"), src, &mut offenders);
        assert_eq!(offenders.len(), 1);
        assert!(offenders[0].contains("walk_op"));
    }

    #[test]
    fn accepts_signature_with_budget() {
        let src = "pub fn walk_op(x: u32, b: &mut Budget) -> u32 { x }\n";
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
