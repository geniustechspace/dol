/// Stable DOL content-address domains.
///
/// Numeric values are part of the canonical contract. Do not renumber or reuse
/// existing values after release.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    RawBytes = 1,
    Utf8 = 2,
    Literal = 3,
    DataType = 4,
    Expr = 10,
    ExprArena = 11,
    Schema = 20,
    Program = 30,
    Operation = 31,
    Stream = 40,
    Pipeline = 50,
    WireEnvelope = 60,
    BackendPlan = 70,
    Manifest = 80,
}

impl Kind {
    pub const fn stable_u16(self) -> u16 {
        self as u16
    }

    pub const fn stable_name(self) -> &'static str {
        match self {
            Self::RawBytes => "raw-bytes",
            Self::Utf8 => "utf8",
            Self::Literal => "literal",
            Self::DataType => "data-type",
            Self::Expr => "expr",
            Self::ExprArena => "expr-arena",
            Self::Schema => "schema",
            Self::Program => "program",
            Self::Operation => "operation",
            Self::Stream => "stream",
            Self::Pipeline => "pipeline",
            Self::WireEnvelope => "wire-envelope",
            Self::BackendPlan => "backend-plan",
            Self::Manifest => "manifest",
        }
    }
}
