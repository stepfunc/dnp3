// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::util::BadWrite;

/// Errors that should never occur, but are preferable to functions that could panic. If one ever
/// happens, it indicates a bug in the library itself
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum LogicError {
    /// A read was out of bounds
    BadRead,
    /// A write was out of bounds
    BadWrite,
    /// A size was out of bounds
    BadSize,
}

/// Errors that can occur when parsing a link-layer frame. What happens next depends on the
/// [`LinkErrorMode`](crate::link::LinkErrorMode): with `Close`, these errors percolate up to the
/// main master/outstation task and close the communication session; with `Discard`, the bad data
/// is discarded. Serial and UDP always discard.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum FrameError {
    /// The first start byte wasn't 0x05
    UnexpectedStart1(u8),
    /// The second start byte wasn't 0x64
    UnexpectedStart2(u8),
    /// The length field was less than the minimum of 5
    BadLength(u8),
    /// The CRC of the frame header didn't match
    BadHeaderCrc,
    /// The CRC of a block of the frame payload didn't match
    BadBodyCrc,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum ParseError {
    BadFrame(FrameError),
    BadLogic(LogicError),
}

/// Errors that end a communication session at the link layer
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum LinkError {
    /// An I/O error on the underlying channel, including the channel closing
    Stdio(std::io::ErrorKind),
    /// A malformed link-layer frame with [`LinkErrorMode::Close`](crate::link::LinkErrorMode::Close)
    BadFrame(FrameError),
    /// A bug in the library
    BadLogic(LogicError),
}

impl From<BadWrite> for LinkError {
    fn from(_: BadWrite) -> Self {
        LinkError::BadLogic(LogicError::BadWrite)
    }
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LinkError::Stdio(kind) => write!(f, "{}", std::io::Error::from(*kind)),
            LinkError::BadFrame(err) => write!(f, "{err}"),
            LinkError::BadLogic(err) => write!(f, "{err}"),
        }
    }
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FrameError::BadBodyCrc => f.write_str("bad CRC value in frame payload"),
            FrameError::BadLength(x) => write!(f, "bad frame length: {x}"),
            FrameError::BadHeaderCrc => f.write_str("bad CRC value in frame header"),
            FrameError::UnexpectedStart1(x) => write!(f, "bad frame start1: {x} != 0x05"),
            FrameError::UnexpectedStart2(x) => write!(f, "bad frame start2: {x} != 0x64"),
        }
    }
}

impl std::fmt::Display for LogicError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LogicError::BadRead => f.write_str("read operation was out-of-bounds"),
            LogicError::BadSize => f.write_str("size was out-of-bounds"),
            LogicError::BadWrite => f.write_str("writer operation was out-of-bounds"),
        }
    }
}

impl From<ParseError> for LinkError {
    fn from(err: ParseError) -> Self {
        match err {
            ParseError::BadFrame(inner) => LinkError::BadFrame(inner),
            ParseError::BadLogic(inner) => LinkError::BadLogic(inner),
        }
    }
}

impl From<LogicError> for LinkError {
    fn from(err: LogicError) -> Self {
        LinkError::BadLogic(err)
    }
}

impl From<scursor::WriteError> for LinkError {
    fn from(_: scursor::WriteError) -> Self {
        LinkError::BadLogic(LogicError::BadWrite)
    }
}

impl From<std::io::Error> for LinkError {
    fn from(err: std::io::Error) -> Self {
        LinkError::Stdio(err.kind())
    }
}
