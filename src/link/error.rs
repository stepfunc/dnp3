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
pub enum LinkError {
    IO(std::io::ErrorKind),
    BadFrame(FrameError),
    BadLogic(LogicError),
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

impl From<WriteError> for LinkError {
    fn from(_: WriteError) -> Self {
        LinkError::BadLogic(LogicError::BadWrite)
    }
}

impl From<std::io::Error> for LinkError {
    fn from(err: std::io::Error) -> Self {
        LinkError::IO(err.kind())
    }
}
