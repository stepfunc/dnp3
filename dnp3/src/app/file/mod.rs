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

pub use permissions::*;

/// File status enumeration used in Group 70 objects
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FileStatus {
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
    Other(u8),
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
            _ => Self::Other(value),
        }
    }

    pub(crate) fn to_u8(self) -> u8 {
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
            FileStatus::Other(x) => x,
        }
    }
}

/// File type enumeration used in Group 70 objects
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FileType {
    /// Directory
    Directory,
    /// Simple file type
    File,
    /// Some other value
    Other(u16),
}

impl FileType {
    fn new(value: u16) -> Self {
        match value {
            0 => Self::Directory,
            1 => Self::File,
            _ => Self::Other(value),
        }
    }

    fn to_u16(self) -> u16 {
        match self {
            FileType::Directory => 0,
            FileType::File => 1,
            FileType::Other(x) => x,
        }
    }
}

fn byte_length(s: &str) -> Result<u16, crate::app::format::Overflow> {
    crate::app::format::to_u16(s.as_bytes().len())
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

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ReadError::NoMoreBytes => f.write_str("Insufficient bytes to represent file data"),
            ReadError::BadOffset { expected, actual } => {
                write!(
                    f,
                    "Expected an offset of {expected} but received {actual} in file data"
                )
            }
            ReadError::Overflow => {
                write!(
                    f,
                    "An overflow occurred while parsing file data indicating a bad encoding"
                )
            }
            ReadError::BadString(err) => {
                write!(
                    f,
                    "File data data expected to be a string contains is not UTF-8 encoded: {err}"
                )
            }
        }
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
