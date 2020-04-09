use crate::app::gen::enums::{FunctionCode, QualifierCode};
use crate::app::gen::variations::variation::Variation;
use crate::app::parse::range::InvalidRange;
use crate::util::cursor::ReadError;
use std::fmt::Formatter;

#[derive(Debug, PartialEq)]
pub enum HeaderParseError {
    UnknownFunction(u8),
    InsufficientBytes,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ObjectParseError {
    UnknownGroupVariation(u8, u8),
    UnknownQualifier(u8),
    InsufficientBytes,
    InvalidRange(u16, u16),
    InvalidQualifierForVariation(Variation, QualifierCode),
    UnsupportedQualifierCode(QualifierCode),
    ZeroLengthOctetData,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RequestValidationError {
    UnexpectedFunction(FunctionCode),
    NonFirFin,
    UnexpectedUnsBit(FunctionCode),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ResponseValidationError {
    UnexpectedFunction(FunctionCode),
    SolicitedResponseWithUnsBit,
    UnsolicitedResponseWithoutUnsBit,
}

impl std::fmt::Display for HeaderParseError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            HeaderParseError::UnknownFunction(x) => write!(f, "unknown function: {:?}", x),
            HeaderParseError::InsufficientBytes => {
                write!(f, "insufficient bytes for application layer header")
            }
        }
    }
}

impl std::fmt::Display for ObjectParseError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ObjectParseError::UnknownGroupVariation(g, v) => {
                write!(f, "unknown group/variation: g{}v{}", g, v)
            }
            ObjectParseError::UnknownQualifier(q) => write!(f, "unknown qualifier: 0x{:02X}", q),
            ObjectParseError::InsufficientBytes => {
                f.write_str("insufficient bytes for object header")
            }
            ObjectParseError::InvalidRange(start, stop) => {
                write!(f, "invalid range - start: {} stop: {}", start, stop)
            }
            ObjectParseError::InvalidQualifierForVariation(v, q) => write!(
                f,
                "{:?} may not be used with the qualifier: {} (0x{:02X})",
                v,
                q.description(),
                q.as_u8()
            ),
            ObjectParseError::UnsupportedQualifierCode(q) => write!(
                f,
                "Unsupported qualifier code: {} (0x{:02X})",
                q.description(),
                q.as_u8()
            ),
            ObjectParseError::ZeroLengthOctetData => {
                f.write_str("octet-data may not be zero length")
            }
        }
    }
}

impl std::fmt::Display for RequestValidationError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            RequestValidationError::UnexpectedUnsBit(x) => {
                write!(f, "UNS bit not allowed for function: {:?}", x)
            }
            RequestValidationError::NonFirFin => {
                f.write_str("requests must must have both FIR and FIN set to 1")
            }
            RequestValidationError::UnexpectedFunction(x) => {
                write!(f, "Function {:?} not allowed in requests", x)
            }
        }
    }
}

impl std::fmt::Display for ResponseValidationError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ResponseValidationError::UnexpectedFunction(x) => {
                write!(f, "function {:?} not allowed in responses", x)
            }
            ResponseValidationError::UnsolicitedResponseWithoutUnsBit => {
                f.write_str("unsolicited responses must have the UNS bit set")
            }
            ResponseValidationError::SolicitedResponseWithUnsBit => {
                f.write_str("solicited responses may not have the UNS bit set")
            }
        }
    }
}

impl std::convert::From<ReadError> for ObjectParseError {
    fn from(_: ReadError) -> Self {
        ObjectParseError::InsufficientBytes
    }
}

impl std::convert::From<ReadError> for HeaderParseError {
    fn from(_: ReadError) -> Self {
        HeaderParseError::InsufficientBytes
    }
}

impl std::convert::From<InvalidRange> for ObjectParseError {
    fn from(r: InvalidRange) -> Self {
        ObjectParseError::InvalidRange(r.start, r.stop)
    }
}
