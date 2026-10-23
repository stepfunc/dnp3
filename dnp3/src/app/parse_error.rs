// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

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
    #[deprecated(since = "1.8.0", note = "never returned; will be removed in 2.0")]
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

#[cfg(feature = "ffi")]
pub use cause::ParseErrorCause;
#[cfg(not(feature = "ffi"))]
pub(crate) use cause::ParseErrorCause;

mod cause {
    /// Whether an [`ObjectParseError`](super::ObjectParseError) means the input is invalid or
    /// that the parser doesn't support it
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "ffi", doc(hidden))]
    #[cfg_attr(
        not(feature = "ffi"),
        expect(unreachable_pub, reason = "public only when building the FFI")
    )]
    pub enum ParseErrorCause {
        /// The parser doesn't support what was received, which may be valid
        Unsupported,
        /// What was received is encoded incorrectly
        Invalid,
        /// Either, depending on the case: e.g. a qualifier the specification disallows for a
        /// variation, or one it allows that the parser doesn't support
        UnsupportedOrInvalid,
    }
}

impl From<ObjectParseError> for ParseErrorCause {
    fn from(err: ObjectParseError) -> Self {
        err.cause()
    }
}

impl ObjectParseError {
    /// Whether the error means the input is invalid or that the parser doesn't support it
    pub(crate) fn cause(&self) -> ParseErrorCause {
        match self {
            ObjectParseError::UnknownGroupVariation(_, _)
            | ObjectParseError::UnsupportedFreeFormatCount(_) => ParseErrorCause::Unsupported,
            #[expect(
                deprecated,
                reason = "the mapping must cover every variant until removal"
            )]
            ObjectParseError::UnsupportedQualifierCode(_) => ParseErrorCause::Unsupported,
            ObjectParseError::InvalidQualifierForVariation(_, _) => {
                ParseErrorCause::UnsupportedOrInvalid
            }
            ObjectParseError::UnknownQualifier(_)
            | ObjectParseError::InsufficientBytes
            | ObjectParseError::InvalidRange(_, _)
            | ObjectParseError::ZeroLengthOctetData
            | ObjectParseError::BadAttribute(_)
            | ObjectParseError::BadEncoding => ParseErrorCause::Invalid,
        }
    }
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
            #[expect(deprecated, reason = "Display must cover every variant until removal")]
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn classifies_each_error_by_cause() {
        use ParseErrorCause::*;

        #[expect(
            deprecated,
            reason = "the classification covers every variant until removal"
        )]
        let cases = [
            (ObjectParseError::UnknownGroupVariation(111, 5), Unsupported),
            (
                ObjectParseError::InvalidQualifierForVariation(
                    Variation::Group111(5),
                    QualifierCode::Range8,
                ),
                UnsupportedOrInvalid,
            ),
            (
                ObjectParseError::UnsupportedQualifierCode(QualifierCode::Range8),
                Unsupported,
            ),
            (ObjectParseError::UnsupportedFreeFormatCount(2), Unsupported),
            (ObjectParseError::UnknownQualifier(0xFF), Invalid),
            (ObjectParseError::InsufficientBytes, Invalid),
            (ObjectParseError::InvalidRange(2, 1), Invalid),
            (ObjectParseError::ZeroLengthOctetData, Invalid),
            (
                ObjectParseError::BadAttribute(AttrParseError::ReadError),
                Invalid,
            ),
            (ObjectParseError::BadEncoding, Invalid),
        ];

        for (err, expected) in cases {
            assert_eq!(err.cause(), expected, "{err:?}");
        }
    }
}
