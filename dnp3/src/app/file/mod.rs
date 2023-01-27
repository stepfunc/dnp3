use std::str::Utf8Error;

mod g70v2;
mod g70v3;
mod g70v4;
mod g70v5;
mod g70v6;
mod g70v7;
mod g70v8;
mod permissions;

pub(crate) use g70v2::*;
pub(crate) use g70v3::*;
pub(crate) use g70v4::*;
pub(crate) use g70v5::*;
pub(crate) use g70v6::*;
pub(crate) use g70v7::*;
pub(crate) use g70v8::*;
pub(crate) use permissions::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum FileStatus {
    /// Requested operation was successful
    Success,
    /// Permission was denied due to improper authentication key, user name, or password
    PermissionDenied,
    /// An unsupported or unknown operation mode was requested
    InvalidMode,
    /// Requested file does not exist
    FileNotFound,
    /// Requested file is already in use
    FileLocked,
    /// File could not be opened because of limit on the number of open files
    TooManyOpen,
    /// There is no file opened with the handle in the request
    InvalidHandle,
    /// Outstation is unable to negotiate a suitable write block size
    WriteBlockSize,
    /// Communications were lost or cannot be establishes with end device where file resides
    CommLost,
    /// An abort request was unsuccessful because the outstation is unable or not programmed to abort
    CannotAbort,
    /// File handle does not reference an opened file
    NotOpened,
    /// File closed due to inactivity timeout
    HandleExpired,
    /// Too much file data was received for outstation to process
    BufferOverrun,
    /// An error occurred in the file processing that prevents any further activity with this file
    Fatal,
    /// The block number did not have the expected sequence number
    BlockSeq,
    /// Some other error not list here occurred. Optional text may provide further explanation.
    Undefined,
    /// Used to capture reserved values
    Reserved(u8),
}

impl FileStatus {
    fn new(value: u8) -> Self {
        match value {
            0 => Self::Success,
            1 => Self::PermissionDenied,
            2 => Self::InvalidMode,
            3 => Self::FileNotFound,
            4 => Self::FileLocked,
            5 => Self::TooManyOpen,
            6 => Self::InvalidHandle,
            7 => Self::WriteBlockSize,
            8 => Self::CommLost,
            9 => Self::CannotAbort,
            16 => Self::NotOpened,
            17 => Self::HandleExpired,
            18 => Self::BufferOverrun,
            19 => Self::Fatal,
            20 => Self::BlockSeq,
            255 => Self::Undefined,
            _ => Self::Reserved(value),
        }
    }

    fn to_u8(self) -> u8 {
        match self {
            FileStatus::Success => 0,
            FileStatus::PermissionDenied => 1,
            FileStatus::InvalidMode => 2,
            FileStatus::FileNotFound => 3,
            FileStatus::FileLocked => 4,
            FileStatus::TooManyOpen => 5,
            FileStatus::InvalidHandle => 6,
            FileStatus::WriteBlockSize => 7,
            FileStatus::CommLost => 8,
            FileStatus::CannotAbort => 9,
            FileStatus::NotOpened => 16,
            FileStatus::HandleExpired => 17,
            FileStatus::BufferOverrun => 18,
            FileStatus::Fatal => 19,
            FileStatus::BlockSeq => 20,
            FileStatus::Undefined => 255,
            FileStatus::Reserved(x) => x,
        }
    }
}

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
