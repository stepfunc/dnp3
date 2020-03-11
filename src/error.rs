use crate::error::TransmitError::BadLogic;
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
pub enum TransmitError {
    IO(std::io::ErrorKind),
    BadLogic(LogicError),
}

impl std::convert::From<LogicError> for TransmitError {
    fn from(err: LogicError) -> Self {
        BadLogic(err)
    }
}

impl std::convert::From<WriteError> for TransmitError {
    fn from(_: WriteError) -> Self {
        BadLogic(LogicError::BadWrite)
    }
}

impl std::convert::From<std::io::Error> for TransmitError {
    fn from(err: std::io::Error) -> Self {
        TransmitError::IO(err.kind())
    }
}
