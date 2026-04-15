//! Bind parameter styles and counter for dialect-aware SQL rendering.

/// How bind parameters are formatted in SQL.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "style", content = "prefix"))]
pub enum ParamStyle {
    /// PostgreSQL: `$1, $2, $3`
    Numbered(String),
    /// MySQL/SQLite: `?`
    Positional,
    /// SQL Server: `@p1, @p2, @p3`
    NamedNumbered(String),
    /// Oracle: `:1, :2, :3`
    ColonNumbered,
}

impl ParamStyle {
    pub fn postgres() -> Self {
        Self::Numbered("$".to_string())
    }

    pub fn mysql() -> Self {
        Self::Positional
    }

    pub fn mssql() -> Self {
        Self::NamedNumbered("@p".to_string())
    }

    pub fn oracle() -> Self {
        Self::ColonNumbered
    }
}

/// Tracks parameter index during SQL rendering and emits dialect-correct placeholders.
///
/// Not Clone — prevents accidental reuse of counters.
pub struct ParamCounter {
    style: ParamStyle,
    current: usize,
}

impl ParamCounter {
    pub fn new(style: &ParamStyle) -> Self {
        Self {
            style: style.clone(),
            current: 0,
        }
    }

    /// Emit the next placeholder string and advance the counter.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> String {
        self.current += 1;
        self.format(self.current)
    }

    /// Return the current count of parameters emitted so far.
    pub fn count(&self) -> usize {
        self.current
    }

    /// Format a placeholder at a specific 1-based index without advancing.
    pub fn format(&self, idx: usize) -> String {
        match &self.style {
            ParamStyle::Numbered(prefix) => format!("{}{}", prefix, idx),
            ParamStyle::Positional => "?".to_string(),
            ParamStyle::NamedNumbered(prefix) => format!("{}{}", prefix, idx),
            ParamStyle::ColonNumbered => format!(":{}", idx),
        }
    }
}
