//! SARIF emitter for `dol-check` diagnostics.
//!
//! Per `docs/v2_plan.md` §68 (*"`check` runs every validator in one pass and
//! emits SARIF (good DX in IDEs)"*). Converts a slice of
//! [`dol_core::diag::Diagnostic`]s into a [SARIF v2.1.0][sarif] log so that
//! IDEs (VS Code, IntelliJ via the SARIF Viewer extension), GitHub Code
//! Scanning, and Azure DevOps can render DOL findings without
//! implementing a custom parser.
//!
//! The emitter is hand-written rather than serde-driven: the SARIF subset
//! we need is small, fixed, and we want the module to compile in
//! `no_std + alloc` mode without pulling in `serde_json`. This also keeps
//! the v2 invariant that DOL crates do not depend on `serde_json` at
//! build time.
//!
//! [sarif]: https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html
//!
//! # Example
//!
//! ```
//! use dol_check::sarif::to_sarif;
//! use dol_core::diag::{Diagnostic, code};
//! use dol_core::span::Span;
//!
//! let diags = [Diagnostic::error(code::TYPE_MISMATCH, Span::NONE, "bad")];
//! let log = to_sarif(&diags);
//! assert!(log.contains("\"version\":\"2.1.0\""));
//! assert!(log.contains("DOL1001"));
//! ```

use alloc::string::String;
use core::fmt::Write as _;

use dol_core::diag::{Diagnostic, Severity};
use dol_core::span::Span;

/// Render a slice of diagnostics as a SARIF v2.1.0 log document.
///
/// Output is a valid, single-`run` SARIF JSON string suitable for writing
/// straight to a `.sarif` file. The driver is reported as `dol-check`
/// with the package version embedded at compile time
/// (`CARGO_PKG_VERSION`).
///
/// Each [`Diagnostic`] becomes one SARIF `result`:
///
/// - `ruleId` is the canonical `DOLNNNN` form of the [`ErrorCode`].
/// - `level` is `error` / `warning` / `note` per [`Severity`]; SARIF has
///   no dedicated *lint* level so [`Severity::Lint`] also maps to
///   `warning` (matching how clippy is rendered in IDEs).
/// - `message.text` is the diagnostic's short message.
/// - `locations` is populated only when the [`Span`]'s file is not
///   `FileId::NONE`. SARIF requires absolute or relative URIs; we use
///   the synthetic scheme `dol-file://{file_id}` because DOL spans carry
///   numeric file ids rather than paths. Tooling that owns the file
///   table (the parser, an LSP server, …) can rewrite this URI to a real
///   `file://` path before display.
///
/// [`ErrorCode`]: dol_core::diag::ErrorCode
pub fn to_sarif(diagnostics: &[Diagnostic]) -> String {
    let mut s = String::new();
    s.push_str("{\"version\":\"2.1.0\",\"$schema\":\"https://docs.oasis-open.org/sarif/sarif/v2.1.0/schemas/sarif-schema-2.1.0.json\",\"runs\":[{");

    // tool.driver
    s.push_str("\"tool\":{\"driver\":{\"name\":\"dol-check\",\"informationUri\":\"https://github.com/geniustechspace/dol\",\"version\":\"");
    s.push_str(env!("CARGO_PKG_VERSION"));
    s.push_str("\"}},");

    // results[]
    s.push_str("\"results\":[");
    let mut first = true;
    for d in diagnostics {
        if !first {
            s.push(',');
        }
        first = false;
        write_result(&mut s, d);
    }
    s.push_str("]}]}");
    s
}

/// Emit a single SARIF `result` object for `d`.
fn write_result(out: &mut String, d: &Diagnostic) {
    out.push_str("{\"ruleId\":\"");
    // ErrorCode's `Display` impl writes the canonical `DOLNNNN` form
    // straight into a formatter without allocating; we mirror that here
    // through `core::fmt::Write` so we never produce a temporary
    // `String` per diagnostic.
    let _ = write!(out, "{}", d.code);
    out.push_str("\",\"level\":\"");
    out.push_str(severity_to_level(d.severity));
    out.push_str("\",\"message\":{\"text\":\"");
    escape_json_into(out, &d.message);
    out.push_str("\"}");

    if !d.span.is_none() {
        write_locations(out, d.span);
    }

    out.push('}');
}

/// SARIF `level` enum — `error`, `warning`, `note`. SARIF has no
/// dedicated `lint` level, so [`Severity::Lint`] folds into `warning`,
/// which matches how clippy is rendered by GitHub Code Scanning and the
/// SARIF Viewer extension.
fn severity_to_level(sev: Severity) -> &'static str {
    match sev {
        Severity::Error => "error",
        Severity::Warning | Severity::Lint => "warning",
        Severity::Note => "note",
    }
}

/// Emit a SARIF `locations` array containing the single physical
/// location implied by `span`. Caller has already verified
/// `span.file() != FileId::NONE`.
fn write_locations(out: &mut String, span: Span) {
    out.push_str(
        ",\"locations\":[{\"physicalLocation\":{\"artifactLocation\":{\"uri\":\"dol-file://",
    );
    let _ = write!(out, "{}", span.file().0);
    out.push_str("\"},\"region\":{\"byteOffset\":");
    let _ = write!(out, "{}", span.start());
    out.push_str(",\"byteLength\":");
    let _ = write!(out, "{}", span.length());
    out.push_str("}}}]");
}

/// Escape `s` into `out` per RFC 8259 §7. SARIF is JSON, so the same
/// rules apply: `"`, `\`, and every C0 control character must be
/// emitted as a `\uXXXX` escape (or one of the canonical short forms
/// for `\b \f \n \r \t`). All other characters — including non-ASCII —
/// pass through unchanged.
fn escape_json_into(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            // Other C0 controls have no short form.
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dol_core::diag::code;

    #[test]
    fn empty_results_array_is_valid_sarif() {
        let log = to_sarif(&[]);
        assert!(log.contains("\"version\":\"2.1.0\""));
        assert!(log.contains("\"name\":\"dol-check\""));
        assert!(log.contains("\"results\":[]"));
    }

    #[test]
    fn error_code_renders_as_canonical_dol_nnnn_string() {
        let d = Diagnostic::error(code::TYPE_MISMATCH, Span::NONE, "x");
        let log = to_sarif(&[d]);
        // `code::TYPE_MISMATCH` is the canonical first-layer code 1001.
        assert!(log.contains("\"ruleId\":\"DOL1001\""), "got: {log}");
    }

    #[test]
    fn severity_levels_map_per_sarif_spec() {
        let span = Span::NONE;
        let diags = [
            Diagnostic::error(code::TYPE_MISMATCH, span, "e"),
            Diagnostic::warning(code::TYPE_MISMATCH, span, "w"),
            Diagnostic::lint(code::TYPE_MISMATCH, span, "l"),
            Diagnostic::note(code::TYPE_MISMATCH, span, "n"),
        ];
        let log = to_sarif(&diags);
        assert!(log.contains("\"level\":\"error\""));
        assert!(log.contains("\"level\":\"warning\""));
        assert!(log.contains("\"level\":\"note\""));
        // Lint folds into warning — there must be exactly two occurrences.
        assert_eq!(log.matches("\"level\":\"warning\"").count(), 2);
    }

    #[test]
    fn message_special_characters_are_json_escaped() {
        let d = Diagnostic::error(
            code::TYPE_MISMATCH,
            Span::NONE,
            "quote: \" backslash: \\ newline: \n tab: \t bell: \x07",
        );
        let log = to_sarif(&[d]);
        assert!(log.contains(r#"quote: \" backslash: \\ newline: \n tab: \t bell: \u0007"#));
        // The raw control character must NOT appear in the output.
        assert!(!log.contains('\x07'));
    }

    #[test]
    fn span_with_no_file_omits_locations() {
        let d = Diagnostic::error(code::TYPE_MISMATCH, Span::NONE, "x");
        let log = to_sarif(&[d]);
        assert!(!log.contains("locations"));
    }

    #[test]
    fn span_with_file_emits_byte_offset_and_length() {
        let span = Span::new(dol_core::span::FileId(7), 100, 25);
        let d = Diagnostic::error(code::TYPE_MISMATCH, span, "x");
        let log = to_sarif(&[d]);
        assert!(log.contains("\"uri\":\"dol-file://7\""));
        assert!(log.contains("\"byteOffset\":100"));
        assert!(log.contains("\"byteLength\":25"));
    }
}
