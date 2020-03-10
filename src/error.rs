// these errors should never occur, but they are preferable to using
// functions that could panic
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum LogicError {
    BadRead,
    BadSize,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum FrameError {
    BadLength(u8),
    BadHeaderCRC,
    BadBodyCRC,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ParseError {
    BadFrame(FrameError),
    BadLogic(LogicError),
}
