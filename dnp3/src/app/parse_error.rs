use std::fmt::Formatter;

use crate::app::parse::range::InvalidRange;
use crate::app::sequence::Sequence;
use crate::app::variations::Variation;
use crate::app::{FunctionCode, QualifierCode};
use crate::util::cursor::ReadError;

/// errors that occur when parsing an application layer header
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HeaderParseError {
    /// unknown function code
    UnknownFunction(Sequence, u8),
    /// insufficient bytes for a header
    InsufficientBytes,
}

/// errors that occur when parsing object headers
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ObjectParseError {
    /// unknown group and variation
    UnknownGroupVariation(u8, u8),
    /// unknown qualifier code
    UnknownQualifier(u8),
    /// insufficient bytes for object header or specified object values
    InsufficientBytes,
    /// range where stop < start
    InvalidRange(u16, u16),
    /// specified variation and qualifier code are invalid or not supported
    InvalidQualifierForVariation(Variation, QualifierCode),
    /// specified qualifier code is not supported
    UnsupportedQualifierCode(QualifierCode),
    /// response containing zero-length octet data disallowed by the specification
    ZeroLengthOctetData,
}

/// errors that occur when interpreting a header as a request header
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RequestValidationError {
    /// function code not allowed in requests
    UnexpectedFunction(FunctionCode),
    /// request with either FIR or FIN == 0
    NonFirFin,
    /// request with an UNS bit that doesn't match the function code (only allowed in Confirm)
    UnexpectedUnsBit(FunctionCode),
}

/// errors that occur when interpreting a header as a response header
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ResponseValidationError {
    /// function code not allowed in responses
    UnexpectedFunction(FunctionCode),
    /// solicited response with UNS == 1
    SolicitedResponseWithUnsBit,
    /// unsolicited response without UNS == 0
    UnsolicitedResponseWithoutUnsBit,
    /// unsolicited response with either FIR or FIN == 0
    UnsolicitedResponseWithoutFirAndFin,
}

impl std::fmt::Display for HeaderParseError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            HeaderParseError::UnknownFunction(_seq, x) => write!(f, "unknown function: {:?}", x),
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
                f.write_str("tasks must must have both FIR and FIN set to 1")
            }
            RequestValidationError::UnexpectedFunction(x) => {
                write!(f, "Function {:?} not allowed in tasks", x)
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
            ResponseValidationError::UnsolicitedResponseWithoutFirAndFin => {
                f.write_str("unsolicited responses must have FIR = 1 and FIN = 1")
            }
        }
    }
}

impl From<ReadError> for ObjectParseError {
    fn from(_: ReadError) -> Self {
        ObjectParseError::InsufficientBytes
    }
}

impl From<ReadError> for HeaderParseError {
    fn from(_: ReadError) -> Self {
        HeaderParseError::InsufficientBytes
    }
}

impl From<InvalidRange> for ObjectParseError {
    fn from(r: InvalidRange) -> Self {
        ObjectParseError::InvalidRange(r.start, r.stop)
    }
}
