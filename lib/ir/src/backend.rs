//! Backend trait and structured [`BackendError`].
//!
//! - [`Backend::compile`] takes a borrowed
//!   [`ProgramRef`] so backends do not need to clone the
//!   arena into every invocation.
//! - [`BackendError`] is `#[non_exhaustive]` and carries a structured
//!   [`dol_core::diag::Diagnostic`] plus optional [`dol_core::span::Span`].
//!   New error kinds (`Capability`, `Extension`, `AclDenied`) can be added
//!   without breaking match arms.

use alloc::string::String;

use dol_core::diag::Diagnostic;
use dol_core::span::Span;

use crate::program_ref::ProgramRef;
use crate::capabilities::CapabilityCheck;

extern crate alloc;

/// Concrete error type returned by DOL IR backends.
///
/// Variants are `#[non_exhaustive]` to allow future additions without
/// breaking downstream match arms.
///
/// # Examples
///
/// ```
/// use dol_ir::backend::BackendError;
///
/// let err = BackendError::unsupported("MERGE not supported by this backend");
/// assert!(err.diagnostic().message.contains("MERGE"));
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum BackendError {
    /// The operation type / target combination is not supported.
    Unsupported {
        /// Structured diagnostic describing the unsupported scenario.
        diag: Diagnostic,
        /// Optional source span where the operation was declared.
        span: Option<Span>,
    },
    /// A required value was missing or null.
    MissingValue {
        /// Structured diagnostic describing what was missing.
        diag: Diagnostic,
        /// Optional source span for the missing value.
        span: Option<Span>,
    },
    /// Generic rendering / compilation error.
    Render {
        /// Structured diagnostic describing the render failure.
        diag: Diagnostic,
        /// Optional source span that triggered the error.
        span: Option<Span>,
    },
    /// A capability the program requires is not provided by the backend.
    Capability {
        /// The capability check that failed.
        check: CapabilityCheck,
        /// Structured diagnostic describing the missing capability.
        diag: Diagnostic,
        /// Optional source span referencing the capability-dependent code.
        span: Option<Span>,
    },
    /// The program references an extension the backend cannot resolve.
    Extension {
        /// Structured diagnostic describing the unresolved extension.
        diag: Diagnostic,
        /// Optional source span where the extension was referenced.
        span: Option<Span>,
    },
    /// An ACL policy denied the operation.
    AclDenied {
        /// Structured diagnostic describing the ACL denial.
        diag: Diagnostic,
        /// Optional source span where the denied operation was declared.
        span: Option<Span>,
    },
}

impl BackendError {
    /// Construct an `Unsupported` error from a free-form message. The
    /// diagnostic uses [`dol_core::diag::code::UNSUPPORTED_BY_BACKEND`].
    pub fn unsupported(msg: impl Into<String>) -> Self {
        BackendError::Unsupported {
            diag: Diagnostic::error(
                dol_core::diag::code::UNSUPPORTED_BY_BACKEND,
                Span::NONE,
                msg.into(),
            ),
            span: None,
        }
    }

    /// Construct a `MissingValue` error.
    ///
    /// Falls back to [`dol_core::diag::code::INTERNAL_ERROR`] rather than
    /// [`dol_core::diag::code::VALUE_OUT_OF_RANGE`]: a missing/null required
    /// value is a contract violation, not an out-of-range numeric.
    /// Downstream tooling that keys off [`Diagnostic::code`] should match
    /// the variant rather than the code for this error.
    pub fn missing_value(msg: impl Into<String>) -> Self {
        BackendError::MissingValue {
            diag: Diagnostic::error(dol_core::diag::code::INTERNAL_ERROR, Span::NONE, msg.into()),
            span: None,
        }
    }

    /// Construct a `Render` error.
    pub fn render(msg: impl Into<String>) -> Self {
        BackendError::Render {
            diag: Diagnostic::error(dol_core::diag::code::INTERNAL_ERROR, Span::NONE, msg.into()),
            span: None,
        }
    }

    /// Construct a `Capability` error from a [`CapabilityCheck`] and message.
    pub fn capability(check: CapabilityCheck, msg: impl Into<String>) -> Self {
        BackendError::Capability {
            check,
            diag: Diagnostic::error(
                dol_core::diag::code::MISSING_CAPABILITY,
                Span::NONE,
                msg.into(),
            ),
            span: None,
        }
    }

    /// Underlying diagnostic.
    pub fn diagnostic(&self) -> &Diagnostic {
        match self {
            BackendError::Unsupported { diag, .. }
            | BackendError::MissingValue { diag, .. }
            | BackendError::Render { diag, .. }
            | BackendError::Capability { diag, .. }
            | BackendError::Extension { diag, .. }
            | BackendError::AclDenied { diag, .. } => diag,
        }
    }
}

impl core::fmt::Display for BackendError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let d = self.diagnostic();
        write!(f, "[{}] {}", d.code, d.message)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BackendError {}

/// A compiler from a [`ProgramRef`] into a backend-specific artifact.
///
/// The associated [`Backend::Output`] type lets each backend describe its
/// natural result shape: text-oriented backends (SQL, GraphQL, …) can use
/// `String`, while richer backends can return structured types carrying
/// metadata such as parameter bindings, key/value operation descriptors, or
/// storage actions.
///
/// Backends receive a [`ProgramRef`] — a borrowed view over the operation
/// sequence, expression arena, interner, and optional schema catalog.
pub trait Backend {
    /// The concrete artifact produced by this backend.
    type Output;

    /// Compile a borrowed program into the backend output. Backends that
    /// only handle a subset of [`crate::operation::Operation`] variants
    /// should return [`BackendError::unsupported`] for the rest.
    #[allow(clippy::result_large_err)]
    fn compile(&self, program: ProgramRef<'_>) -> Result<Self::Output, BackendError>;
}
