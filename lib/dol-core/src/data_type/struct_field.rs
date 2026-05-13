//! `StructField`: a named, typed field used inside `DataType::Struct`.

use alloc::boxed::Box;

use super::DataType;

/// A named, typed field inside a [`DataType::Struct`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct StructField {
    pub name: Box<str>,
    pub data_type: DataType,
    pub nullable: bool,
}

impl StructField {
    pub fn new(name: impl Into<Box<str>>, data_type: DataType, nullable: bool) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable,
        }
    }
}
