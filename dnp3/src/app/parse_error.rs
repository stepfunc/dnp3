use std::fmt::Formatter;

use crate::app::parse::range::InvalidRange;
use crate::app::sequence::Sequence;
use crate::app::variations::Variation;
use crate::app::{FunctionCode, QualifierCode};

use crate::app::attr::AttrParseError;
use scursor::{ReadError, TrailingBytes};

/// Errors that occur when parsing an application layer header
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HeaderParseError {
    /// Unknown function code
    UnknownFunction(Sequence, u8),
    /// insufficient bytes for a header
    InsufficientBytes,
}

/// Errors that occur when parsing object headers
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ObjectParseError {
    /// Unknown group and variation
    UnknownGroupVariation(u8, u8),
    /// Unknown qualifier code
    UnknownQualifier(u8),
    /// Insufficient bytes for object header or specified object values
    InsufficientBytes,
    /// Range where stop < start
    InvalidRange(u16, u16),
    /// Specified variation and qualifier code are invalid or not supported
    InvalidQualifierForVariation(Variation, QualifierCode),
    /// Specified qualifier code is not supported
    UnsupportedQualifierCode(QualifierCode),
    /// Free format parser can only handle counts of 1
    UnsupportedFreeFormatCount(u8),
    /// Response containing zero-length octet data disallowed by the specification
    ZeroLengthOctetData,
    /// Device attribute parsing error
    BadAttribute(AttrParseError),
    /// Object is not properly encoded
    BadEncoding,
}

impl From<TrailingBytes> for ObjectParseError {
    fn from(_: TrailingBytes) -> Self {
        Self::BadEncoding
    }
}

/// Errors that occur when interpreting a header as a request header
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RequestValidationError {
    /// Function code not allowed in requests
    UnexpectedFunction(FunctionCode),
    /// Request with either FIR or FIN == 0
    NonFirFin,
    /// Request with an UNS bit that doesn't match the function code (only allowed in Confirm)
    UnexpectedUnsBit(FunctionCode),
}

/// Errors that occur when interpreting a header as a response header
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResponseValidationError {
    /// Function code not allowed in responses
    UnexpectedFunction(FunctionCode),
    /// Solicited response with UNS == 1
    SolicitedResponseWithUnsBit,
    /// Unsolicited response without UNS == 0
    UnsolicitedResponseWithoutUnsBit,
    /// Unsolicited response with either FIR or FIN == 0
    UnsolicitedResponseWithoutFirAndFin,
}

impl std::fmt::Display for HeaderParseError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            HeaderParseError::UnknownFunction(_seq, x) => write!(f, "unknown function: {x:?}"),
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
                write!(f, "unknown group/variation: g{g}v{v}")
            }
            ObjectParseError::UnknownQualifier(q) => write!(f, "unknown qualifier: 0x{q:02X}"),
            ObjectParseError::InsufficientBytes => {
                f.write_str("insufficient bytes for object header")
            }
            ObjectParseError::InvalidRange(start, stop) => {
                write!(f, "invalid range - start: {start} stop: {stop}")
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
            ObjectParseError::BadAttribute(x) => write!(f, "{x}"),
            ObjectParseError::BadEncoding => f.write_str("Object is not properly encoded"),
            ObjectParseError::UnsupportedFreeFormatCount(x) => {
                write!(f, "Unsupported free-format count: {x}")
            }
        }
    }
}

impl std::fmt::Display for RequestValidationError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            RequestValidationError::UnexpectedUnsBit(x) => {
                write!(f, "UNS bit not allowed for function: {x:?}")
            }
            RequestValidationError::NonFirFin => {
                f.write_str("tasks must must have both FIR and FIN set to 1")
            }
            RequestValidationError::UnexpectedFunction(x) => {
                write!(f, "Function {x:?} not allowed in tasks")
            }
        }
    }
}

impl std::fmt::Display for ResponseValidationError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ResponseValidationError::UnexpectedFunction(x) => {
                write!(f, "function {x:?} not allowed in responses")
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

impl From<AttrParseError> for ObjectParseError {
    fn from(x: AttrParseError) -> Self {
        Self::BadAttribute(x)
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
