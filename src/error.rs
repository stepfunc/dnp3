use crate::error::Error::BadLogic;
use crate::util::cursor::WriteError;

// these errors should never occur, but they are preferable to using
// functions that could panic
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum LogicError {
    BadRead,
    BadWrite,
    BadSize,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum FrameError {
    UnexpectedStart1(u8),
    UnexpectedStart2(u8),
    BadLength(u8),
    BadHeaderCRC,
    BadBodyCRC,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ParseError {
    BadFrame(FrameError),
    BadLogic(LogicError),
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Error {
    IO(std::io::ErrorKind),
    BadFrame(FrameError),
    BadLogic(LogicError),
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        match err {
            ParseError::BadFrame(inner) => Error::BadFrame(inner),
            ParseError::BadLogic(inner) => BadLogic(inner),
        }
    }
}

impl From<LogicError> for Error {
    fn from(err: LogicError) -> Self {
        BadLogic(err)
    }
}

impl From<WriteError> for Error {
    fn from(_: WriteError) -> Self {
        BadLogic(LogicError::BadWrite)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IO(err.kind())
    }
}
