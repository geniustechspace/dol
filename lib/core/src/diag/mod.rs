//! # `diag` — structured diagnostics
//!
//! Every DOL component that validates user input (`dol-check`, `dol-wire`,
//! `dol-schema`, …) emits [`Diagnostic`] values rather than panicking.
//!
//! A diagnostic is:
//! - a stable [`Code`] (so external tooling can pin behaviour),
//! - a [`Severity`],
//! - a primary [`crate::span::Span`],
//! - a short human message,
//! - zero or more secondary [`Label`]s, [`Note`]s, and [`FixIt`] hints.
//!
//! The catalogue of built-in codes lives in [`code`]. Downstream code may
//! mint its own codes by passing a `&'static str`.

use crate::span::Span;
use smallvec::SmallVec;

pub mod code;

pub use code::Code;

/// Diagnostic severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Severity {
    /// Hard error: the program cannot be executed.
    Error,
    /// Soft warning: the program is well-formed but suspicious.
    Warning,
    /// Style or best-practice suggestion.
    Lint,
    /// Informational note that supplements another diagnostic.
    Note,
}

/// A secondary highlight pointing at a related span.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Label {
    /// Range of source this label calls attention to.
    pub span: Span,
    /// Short human-readable explanation of the label.
    pub text: alloc::string::String,
}

/// A free-form supplementary note.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Note(pub alloc::string::String);

/// A machine-applicable fix-it hint: replace `span` with `replacement`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct FixIt {
    /// Range to replace.
    pub span: Span,
    /// Text to substitute in.
    pub replacement: alloc::string::String,
    /// Why the suggestion is being made.
    pub message: alloc::string::String,
}

/// A single diagnostic.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Diagnostic {
    /// Stable diagnostic code (e.g. `"DOL0042"`).
    pub code: Code,
    /// Severity level.
    pub severity: Severity,
    /// Primary span for the diagnostic.
    pub span: Span,
    /// Short, single-line human message.
    pub message: alloc::string::String,
    /// Secondary labels.
    pub labels: SmallVec<[Label; 2]>,
    /// Supplementary notes.
    pub notes: SmallVec<[Note; 2]>,
    /// Suggested fix-its.
    pub fixits: SmallVec<[FixIt; 1]>,
}

impl Diagnostic {
    /// Build a new error diagnostic.
    pub fn error(code: Code, span: Span, message: impl Into<alloc::string::String>) -> Self {
        Self::new(Severity::Error, code, span, message)
    }

    /// Build a new warning diagnostic.
    pub fn warning(code: Code, span: Span, message: impl Into<alloc::string::String>) -> Self {
        Self::new(Severity::Warning, code, span, message)
    }

    /// Build a new lint diagnostic.
    pub fn lint(code: Code, span: Span, message: impl Into<alloc::string::String>) -> Self {
        Self::new(Severity::Lint, code, span, message)
    }

    /// Build a new note diagnostic.
    pub fn note(code: Code, span: Span, message: impl Into<alloc::string::String>) -> Self {
        Self::new(Severity::Note, code, span, message)
    }

    /// Construct with explicit severity.
    pub fn new(
        severity: Severity,
        code: Code,
        span: Span,
        message: impl Into<alloc::string::String>,
    ) -> Self {
        Self {
            code,
            severity,
            span,
            message: message.into(),
            labels: SmallVec::new(),
            notes: SmallVec::new(),
            fixits: SmallVec::new(),
        }
    }

    /// Attach a secondary label, returning `self`.
    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    /// Attach a note, returning `self`.
    pub fn with_note(mut self, note: impl Into<alloc::string::String>) -> Self {
        self.notes.push(Note(note.into()));
        self
    }

    /// Attach a fix-it, returning `self`.
    pub fn with_fixit(mut self, fixit: FixIt) -> Self {
        self.fixits.push(fixit);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_diagnostic() {
        let d = Diagnostic::error(code::TYPE_MISMATCH, Span::NONE, "expected Int, found Text")
            .with_note("widening from Int to Text is not implicit");
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.code, code::TYPE_MISMATCH);
        assert_eq!(d.notes.len(), 1);
    }
}
