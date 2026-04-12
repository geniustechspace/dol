//! Binary and unary operators for DOL expressions.

/// Binary operators for expression composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Comparison
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    // Logical
    And,
    Or,
    // Pattern
    Like,
    ILike,
    // String
    Concat,
}

/// Unary operators for expression composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}
