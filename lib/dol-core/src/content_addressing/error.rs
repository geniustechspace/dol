use core::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EncodeError {
    BudgetExceeded,
    DepthExceeded,
    ObjectTooLarge,
    UnsupportedVersion,
    NonCanonicalInput,
    SinkFailed,
}

impl fmt::Display for EncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BudgetExceeded => formatter.write_str("canonical encoding budget exceeded"),
            Self::DepthExceeded => formatter.write_str("canonical encoding depth exceeded"),
            Self::ObjectTooLarge => {
                formatter.write_str("object is too large to encode canonically")
            }
            Self::UnsupportedVersion => {
                formatter.write_str("unsupported canonical encoding version")
            }
            Self::NonCanonicalInput => formatter.write_str("input is not in canonical form"),
            Self::SinkFailed => formatter.write_str("canonical sink failed"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentAddressError {
    Encode(EncodeError),
    Collision,
    CorruptObject,
    KindMismatch,
    UnsupportedWidth,
    UnsupportedHashAlgorithm,
    UnknownReference,
    ObjectTooLarge,
    Io,
}

impl From<EncodeError> for ContentAddressError {
    fn from(error: EncodeError) -> Self {
        Self::Encode(error)
    }
}

impl fmt::Display for ContentAddressError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode(error) => error.fmt(formatter),
            Self::Collision => formatter.write_str("content address collision detected"),
            Self::CorruptObject => formatter.write_str("content addressed object is corrupt"),
            Self::KindMismatch => formatter.write_str("content address kind mismatch"),
            Self::UnsupportedWidth => formatter.write_str("unsupported content address width"),
            Self::UnsupportedHashAlgorithm => {
                formatter.write_str("unsupported content address hash algorithm")
            }
            Self::UnknownReference => formatter.write_str("unknown content addressed reference"),
            Self::ObjectTooLarge => formatter.write_str("content addressed object is too large"),
            Self::Io => formatter.write_str("content address storage I/O error"),
        }
    }
}
