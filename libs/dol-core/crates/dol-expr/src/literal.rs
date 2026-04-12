//! Literal values in DOL expressions.

/// A literal value in a DOL expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    /// UUID stored as string representation.
    Uuid(String),
    /// Raw bytes.
    Bytes(Vec<u8>),
    /// Decimal stored as string to preserve precision.
    Decimal(String),
}

impl From<&str> for Literal {
    fn from(s: &str) -> Self {
        Literal::String(s.to_string())
    }
}

impl From<String> for Literal {
    fn from(s: String) -> Self {
        Literal::String(s)
    }
}

impl From<i64> for Literal {
    fn from(v: i64) -> Self {
        Literal::Int(v)
    }
}

impl From<i32> for Literal {
    fn from(v: i32) -> Self {
        Literal::Int(v as i64)
    }
}

impl From<f64> for Literal {
    fn from(v: f64) -> Self {
        Literal::Float(v)
    }
}

impl From<bool> for Literal {
    fn from(v: bool) -> Self {
        Literal::Bool(v)
    }
}
