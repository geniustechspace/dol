//! `Display` implementation for `DataType`.
//!
//! Produces a human-readable type name suitable for error messages,
//! schema descriptions, and DOL DDL output.

use core::fmt;

use super::DataType;

impl fmt::Display for DataType {
    /// Produces a human-readable type name suitable for error messages,
    /// schema descriptions, and DOL DDL output.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Char(n) => write!(f, "CHAR({n})"),
            Self::Varchar(Some(n)) => write!(f, "VARCHAR({n})"),
            Self::Varchar(None) => write!(f, "TEXT"),
            Self::Text => write!(f, "TEXT"),
            Self::Binary(n) => write!(f, "BINARY({n})"),
            Self::Varbinary(Some(n)) => write!(f, "VARBINARY({n})"),
            Self::Varbinary(None) => write!(f, "BLOB"),
            Self::Bit(n) => write!(f, "BIT({n})"),
            Self::Varbit(Some(n)) => write!(f, "VARBIT({n})"),
            Self::Varbit(None) => write!(f, "VARBIT"),
            Self::Decimal {
                precision: Some(p),
                scale: Some(s),
            } => write!(f, "NUMERIC({p},{s})"),
            Self::Decimal {
                precision: Some(p),
                scale: None,
            } => write!(f, "NUMERIC({p})"),
            Self::Decimal { .. } => write!(f, "NUMERIC"),
            Self::Time { precision: 6 } | Self::Time { .. } => {
                if let Self::Time { precision } = self {
                    write!(f, "TIME({precision})")
                } else {
                    unreachable!()
                }
            }
            Self::DateTime { precision } => write!(f, "DATETIME({precision})"),
            Self::TimestampTz { precision } => write!(f, "TIMESTAMPTZ({precision})"),
            Self::Cidr { prefix_len } => write!(f, "CIDR(/{prefix_len})"),
            Self::Array(e) => write!(f, "{e}[]"),
            Self::Set(e) => write!(f, "SET<{e}>"),
            Self::Map { value } => write!(f, "MAP<TEXT,{value}>"),
            Self::Range(e) => write!(f, "RANGE<{e}>"),
            Self::Tuple(ts) => {
                write!(f, "TUPLE(")?;
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{t}")?;
                }
                write!(f, ")")
            }
            Self::Struct(fields) => {
                write!(f, "STRUCT(")?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    let null = if field.nullable { "" } else { " NOT NULL" };
                    write!(f, "{} {}{null}", field.name, field.data_type)?;
                }
                write!(f, ")")
            }
            Self::Enum(variants) => {
                write!(f, "ENUM(")?;
                for (i, v) in variants.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "'{v}'")?;
                }
                write!(f, ")")
            }
            Self::TypeRef(name) => write!(f, "{name}"),
            Self::Extension { name, params } => {
                write!(f, "{name}")?;
                if !params.is_empty() {
                    write!(f, "(")?;
                    for (i, p) in params.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{p}")?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            other => f.write_str(other.type_name().to_ascii_uppercase().as_str()),
        }
    }
}
