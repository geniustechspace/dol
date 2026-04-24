use crate::statement::Statement;

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

pub trait Backend {
    fn compile(&self, stmt: &Statement) -> Result<String, BackendError>;
}
