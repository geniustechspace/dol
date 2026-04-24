use crate::statement::Statement;

pub trait BackendError: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static {}

pub trait Backend {
    type Error: BackendError;

    fn compile(&self, stmt: &Statement) -> Result<String, Self::Error>;
}
