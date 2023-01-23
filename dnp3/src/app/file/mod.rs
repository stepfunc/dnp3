use std::str::Utf8Error;

mod g70v2;
mod g70v3;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Overflow;

impl From<Overflow> for WriteError {
    fn from(_: Overflow) -> Self {
        WriteError::Overflow
    }
}

fn to_u16<X: TryInto<u16>>(x: X) -> Result<u16, Overflow> {
    x.try_into().map_err(|_| Overflow)
}

fn length(s: &str) -> Result<u16, Overflow> {
    to_u16(s.len())
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReadError {
    /// No more data
    NoMoreBytes,
    /// Field has a bad offset in the encoding
    BadOffset { expected: u16, actual: u16 },
    /// The encoding is bad because it requires that a value overflows the u16 representation
    Overflow,
    /// A string is not UTF8 encoded
    BadString(Utf8Error),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum WriteError {
    /// Cursor error
    WriteError(scursor::WriteError),
    /// The provided data would overflow the u16 representation
    Overflow,
}

impl From<scursor::WriteError> for WriteError {
    fn from(value: scursor::WriteError) -> Self {
        Self::WriteError(value)
    }
}

impl From<scursor::ReadError> for ReadError {
    fn from(_: scursor::ReadError) -> Self {
        Self::NoMoreBytes
    }
}

impl From<Utf8Error> for ReadError {
    fn from(value: Utf8Error) -> Self {
        Self::BadString(value)
    }
}
