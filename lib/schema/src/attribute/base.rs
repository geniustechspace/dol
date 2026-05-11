use crate::reference::Reference;
use dol_core::DataType;
use dol_expr::Expr;

#[derive(Debug, Clone)]
pub struct Attribute {
    /// The data type of this attribute.
    pub data_type: DataType,
    /// Whether this attribute is required (non-nullable).
    pub required: bool,
    /// Whether this attribute must have unique values.
    pub unique: bool,
    /// Whether this attribute is an identity column.
    pub identity: bool,
    /// Whether this attribute is a lookup column.
    pub lookup: bool,
    /// Whether this attribute is auto-assigned.
    pub auto_assign: bool,
    /// The default value expression for this attribute.
    pub default: Option<Expr>,
    /// Reference to another attribute.
    pub reference: Option<Reference>,
    /// Inline invariant expression.
    pub check: Option<Expr<'a>>,
    /// Human-readable description / comment.
    pub comment: Option<Arc<str>>,
}

impl Attribute {
    pub fn new(data_type: DataType) -> Self {
        Self {
            data_type,
            required: false,
            unique: false,
            identity: false,
            lookup: false,
            auto_assign: false,
            default: None,
            reference: None,
            check: None,
            comment: None,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Mark field as nullable (optional in DOL terminology).
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    pub fn identity(mut self) -> Self {
        self.identity = true;
        self.required = true;
        self.unique = true;
        self.auto_assign = true;
        self
    }

    pub fn lookup(mut self) -> Self {
        self.lookup = true;
        self
    }

    pub fn auto_assign(mut self) -> Self {
        self.auto_assign = true;
        self
    }

    pub fn default(mut self, expr: impl Into<Expr>) -> Self {
        self.default = Some(expr.into());
        self
    }

    pub fn reference(mut self, reference: Reference) -> Self {
        self.reference = Some(reference);
        self.lookup = true;
        self
    }

    pub fn check(mut self, check: impl Into<Arc<str>>) -> Self {
        self.check = Some(check.into());
        self
    }

    pub fn comment(mut self, comment: impl Into<Arc<str>>) -> Self {
        self.comment = Some(comment.into());
        self
    }
}
