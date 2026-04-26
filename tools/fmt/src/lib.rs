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
//!   (tx :begin)
//!   (insert :target relation/users)
//!   (tx :commit))
//! ```
//!
//! The exact shape is governed by snapshot tests; downstream tooling should
//! treat any change as a breaking change.

#![deny(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

use core::fmt::Write;

use dol_ir::{Operation, Program};

/// Pretty-print a [`Program`] to a `String`.
pub fn print(program: &Program) -> alloc::string::String {
    let mut out = alloc::string::String::new();
    let _ = write_program(&mut out, program);
    out
}

fn write_program(w: &mut alloc::string::String, p: &Program) -> core::fmt::Result {
    writeln!(w, "(program")?;
    for op in &p.operations {
        w.push_str("  ");
        write_operation(w, op)?;
        w.push('\n');
    }
    write!(w, ")")
}

fn write_operation(w: &mut alloc::string::String, op: &Operation) -> core::fmt::Result {
    let kind = op.kind();
    if let Some(t) = op.primary_target() {
        write!(w, "({:?} :target {:?})", kind, t.kind)
    } else {
        write!(w, "({kind:?})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dol_ir::operation::{Insert, InsertSource};
    use dol_ir::{Locator, Program, Symbol, Target, TargetKind};

    #[test]
    fn empty_program() {
        let op: dol_ir::Operation = Insert {
            target: Target::new(TargetKind::Relation, Locator::new(Symbol::default())),
            source: InsertSource::Bindings,
            returning: None,
        }
        .into();
        let p = Program::from_operation(op);
        let s = print(&p);
        assert!(s.starts_with("(program"));
        assert!(s.contains("Insert"));
        assert!(s.contains("Relation"));
    }
}
