use crate::program::Program;

/// Concrete error type returned by DOL IR backends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    /// The statement type is not supported by this backend.
    Unsupported(String),
    /// A value was missing or null where it was required.
    MissingValue(String),
    /// A generic rendering / compilation error.
    Render(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendError::Unsupported(msg) => write!(f, "unsupported operation: {msg}"),
            BackendError::MissingValue(msg) => write!(f, "missing value: {msg}"),
            BackendError::Render(msg) => write!(f, "render error: {msg}"),
        }
    }
}

impl std::error::Error for BackendError {}

/// A compiler from DOL [`Program`] IR into a backend-specific artifact.
///
/// The associated [`Backend::Output`] type lets each backend describe its
/// natural result shape: text-oriented backends (SQL, GraphQL, ...) can use
/// `String`, while richer backends can return structured types carrying
/// metadata such as parameter bindings, key/value operation descriptors, or
/// storage actions.
///
/// Backends receive a [`Program`] — a [`crate::Statement`] together with the
/// [`dol_expr::ExprArena`] and [`dol_expr::Interner`] needed to resolve any
/// arena references the statement may carry. Backends that only handle
/// fully-owned variants can ignore the arena/interner.
pub trait Backend {
    /// The concrete artifact produced by this backend.
    type Output;

    fn compile(&self, program: &Program) -> Result<Self::Output, BackendError>;
}
