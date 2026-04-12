/// Identifier quoting styles for different SQL dialects.
use serde::Deserialize;

/// How identifiers (table names, column names) are quoted.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub enum QuoteStyle {
    /// PostgreSQL / Oracle / ANSI SQL: `"identifier"`
    DoubleQuote,
    /// MySQL / MariaDB: `` `identifier` ``
    Backtick,
    /// SQL Server: `[identifier]`
    Bracket,
    /// No quoting (identifiers used as-is).
    #[default]
    None,
}

impl QuoteStyle {
    /// Quote an identifier according to the dialect.
    pub fn quote(&self, identifier: &str) -> String {
        match self {
            Self::DoubleQuote => format!("\"{}\"", identifier),
            Self::Backtick => format!("`{}`", identifier),
            Self::Bracket => format!("[{}]", identifier),
            Self::None => identifier.to_string(),
        }
    }
}
