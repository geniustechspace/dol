//! # `dol-fmt` — canonical pretty-printer
//!
//! Renders an IR [`dol_ir::Program`] into a stable, human-readable text form.
//! The output is **not** a parser surface; it exists for debugging,
//! `insta`-style golden tests, and round-trip checks against `dol-wire`.
//!
//! The text form is loosely S-expression-flavoured:
//!
//! ```text
//! (program
//!   (transaction (begin))
//!   (set "search_path" "public")
//!   (transaction (commit)))
//! ```
//!
//! The exact shape is governed by snapshot tests; downstream tooling should
//! treat any change as a breaking change.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use core::fmt::Write;

use dol_ir::{Program, Statement};

/// Pretty-print a [`Program`] to a `String`.
pub fn print(program: &Program) -> alloc::string::String {
    let mut out = alloc::string::String::new();
    let _ = write_program(&mut out, program);
    out
}

fn write_program(w: &mut alloc::string::String, p: &Program) -> core::fmt::Result {
    writeln!(w, "(program")?;
    let stmts = core::slice::from_ref(&p.stmt);
    for stmt in stmts {
        w.push_str("  ");
        write_statement(w, stmt)?;
        w.push('\n');
    }
    write!(w, ")")
}

fn write_statement(w: &mut alloc::string::String, stmt: &Statement) -> core::fmt::Result {
    // We deliberately stick to coarse, statement-level rendering for now;
    // detailed per-variant printing is added incrementally with golden tests.
    match stmt {
        Statement::Query(_) => write!(w, "(query)"),
        Statement::Insert(_) => write!(w, "(insert)"),
        Statement::Update(_) => write!(w, "(update)"),
        Statement::Delete(_) => write!(w, "(delete)"),
        Statement::Upsert(_) => write!(w, "(upsert)"),
        Statement::DefineEntity(_) => write!(w, "(define-entity)"),
        Statement::AlterEntity(_) => write!(w, "(alter-entity)"),
        Statement::DropEntity(_) => write!(w, "(drop-entity)"),
        Statement::DefineLookup(_) => write!(w, "(define-lookup)"),
        Statement::DropLookup(_) => write!(w, "(drop-lookup)"),
        Statement::DefineType(_) => write!(w, "(define-type)"),
        Statement::DropType(_) => write!(w, "(drop-type)"),
        Statement::Grant(_) => write!(w, "(grant)"),
        Statement::Revoke(_) => write!(w, "(revoke)"),
        Statement::DefinePolicy(_) => write!(w, "(define-policy)"),
        Statement::Transaction(_) => write!(w, "(transaction)"),
        Statement::PutObject(_) => write!(w, "(put-object)"),
        Statement::GetObject(_) => write!(w, "(get-object)"),
        Statement::ListObjects(_) => write!(w, "(list-objects)"),
        Statement::ReadFile(_) => write!(w, "(read-file)"),
        Statement::WriteFile(_) => write!(w, "(write-file)"),
        Statement::MoveFile(_) => write!(w, "(move-file)"),
        Statement::Raw(_) => write!(w, "(raw)"),
        Statement::Extension(ext) => write!(w, "(extension :id {:?})", ext.id),
    }
}

extern crate alloc;

#[cfg(test)]
mod tests {
    use super::*;
    use dol_ir::{Program, Statement};

    #[test]
    fn empty_program() {
        let p = Program::from_stmt(Statement::Raw(String::from("--")));
        let s = print(&p);
        assert!(s.starts_with("(program"));
        assert!(s.contains("(raw)"));
    }
}
