use crate::util::BadWrite;

/// these errors should never occur, but they are preferable to using
/// functions that could panic. If they ever were to happen, they indicate
/// a bug in the library itself
#[allow(clippy::enum_variant_names)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum LogicError {
    BadRead,
    BadWrite,
    BadSize,
}

/// Errors that can occur when parsing a link-layer frame. On session-oriented transports,
/// such as TCP/TLS, these errors percolate up to the main master/outstation task and kill
/// the communication session. On serial, they are just discarded.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FrameError {
    UnexpectedStart1(u8),
    UnexpectedStart2(u8),
    BadLength(u8),
    BadHeaderCrc,
    BadBodyCrc,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ParseError {
    BadFrame(FrameError),
    BadLogic(LogicError),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum LinkError {
    Stdio(std::io::ErrorKind),
    BadFrame(FrameError),
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
            FrameError::UnexpectedStart2(x) => write!(f, "bad frame start1: {x} != 0x64"),
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
