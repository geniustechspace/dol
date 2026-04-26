//! Backend trait and structured [`BackendError`].
//!
//! v2 reshapes both:
//!
//! - [`Backend::compile`] takes a borrowed
//!   [`ProgramRef`](crate::ProgramRef) so backends do not need to clone the
//!   arena into every invocation.
//! - [`BackendError`] is `#[non_exhaustive]` and carries a structured
//!   [`dol_core::diag::Diagnostic`] plus optional [`dol_core::span::Span`].
//!   New error kinds (`Capability`, `Extension`, `AclDenied`) are added
//!   without breaking match arms.

use alloc::string::String;

use dol_core::diag::Diagnostic;
use dol_core::span::Span;

use crate::ProgramRef;
use crate::capabilities::CapabilityCheck;

extern crate alloc;

/// Concrete error type returned by DOL IR backends.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum BackendError {
    /// The operation type / target combination is not supported.
    Unsupported {
        diag: Diagnostic,
        span: Option<Span>,
    },
    /// A required value was missing or null.
    MissingValue {
        diag: Diagnostic,
        span: Option<Span>,
    },
    /// Generic rendering / compilation error.
    Render {
        diag: Diagnostic,
        span: Option<Span>,
    },
    /// A capability the program requires is not provided by the backend.
    Capability {
        check: CapabilityCheck,
        diag: Diagnostic,
        span: Option<Span>,
    },
    /// The program references an extension the backend cannot resolve.
    Extension {
        diag: Diagnostic,
        span: Option<Span>,
    },
    /// An ACL policy denied the operation.
    AclDenied {
        diag: Diagnostic,
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
    pub fn missing_value(msg: impl Into<String>) -> Self {
        BackendError::MissingValue {
            diag: Diagnostic::error(
                dol_core::diag::code::VALUE_OUT_OF_RANGE,
                Span::NONE,
                msg.into(),
            ),
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
        write!(f, "[{}] {}", d.code.as_str(), d.message)
    }
}

impl std::error::Error for BackendError {}

/// A compiler from a v2 [`ProgramRef`] into a backend-specific artifact.
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
