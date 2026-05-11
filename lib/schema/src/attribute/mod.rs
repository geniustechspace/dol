pub mod base;

pub use base::Attribute;
use dol_core::DataType;

impl Attribute {
    pub fn boolean() -> Self {
        Self::new(DataType::Boolean)
    }

    pub fn string(max_len: Option<u32>) -> Self {
        match max_len {
            Some(len) => Self::new(DataType::varying_string(len)),
            None => Self::new(DataType::unbounded_string()),
        }
    }

    pub fn int() -> Self {
        Self::new(DataType::Int32)
    }

    pub fn email(max_len: Option<u32>) -> Self {
        Self::new(DataType::varying_string(max_len.unwrap_or(320)))
    }

    pub fn uuid() -> Self {
        Self::new(DataType::Uuid)
    }
}
