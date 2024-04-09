use crate::app::file::{FileStatus, FileType, Permissions};
use crate::app::parse::parser::SingleHeaderError;
use crate::app::{MaybeAsync, ObjectParseError, Shutdown, Timestamp};
use crate::master::TaskError;
use std::fmt::Debug;

/// Credentials for obtaining a file authorization token from the outstation
#[derive(Clone, Debug)]
pub struct FileCredentials {
    /// User name
    pub user_name: String,
    /// Password
    pub password: String,
}

/// Information about a file or directory returned from the outstation
///
/// This is a user-facing representation of Group 70 Variation 7
#[derive(Clone, Debug)]
pub struct FileInfo {
    /// Name of the file or directory
    pub name: String,
    /// File or directory
    pub file_type: FileType,
    /// If a file, this represents its size in bytes. If a directory, this represents the number of
    /// files and directories it contains.
    pub size: u32,
    /// Time of creation as a DNP3 timestamp
    pub time_created: Timestamp,
    /// Permissions as defined in the protocol
    pub permissions: Permissions,
}

/// Configuration related to reading a file
#[derive(Copy, Clone, Debug)]
pub struct FileReadConfig {
    /// Maximum block size requested by the master during the file open
    pub max_block_size: u16,
    /// Maximum file size accepted by the master
    ///
    /// This applies to both:
    ///
    /// 1) The size returned by the outstation in the OPEN operation
    /// 2) The total number of bytes actually returned in subsequent READ operations
    pub max_file_size: usize,
}

/// Files can be opened for writing by creating/truncating or appending to an existing file
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FileWriteMode {
    /// Specifies that the file is to be opened for writing. If it already exists, the file is truncated to zero length
    Write,
    /// Specifies that the file is to be opened for writing, appending to the end of the file
    Append,
}

impl From<FileWriteMode> for FileMode {
    fn from(value: FileWriteMode) -> Self {
        match value {
            FileWriteMode::Write => Self::Write,
            FileWriteMode::Append => Self::Append,
        }
    }
}

/// Configuration related to reading a directory
///
///
#[derive(Copy, Clone, Debug)]
pub struct DirReadConfig {
    /// Maximum block size requested by the master during the file open
    pub max_block_size: u16,
    /// Maximum number of bytes that may be accumulated while reading
    /// directory information
    pub max_file_size: usize,
}

impl From<DirReadConfig> for FileReadConfig {
    fn from(value: DirReadConfig) -> Self {
        Self {
            max_block_size: value.max_block_size,
            max_file_size: value.max_file_size,
        }
    }
}

impl FileReadConfig {
    /// Creates a new configuration with maximum values for the file size and max file bytes
    pub fn new() -> Self {
        Self {
            max_block_size: u16::MAX,
            max_file_size: usize::MAX,
        }
    }
}

impl DirReadConfig {
    /// Default maximum number of bytes read
    pub const DEFAULT_MAX_SIZE: usize = 2048;

    /// Creates a new configuration with maximum values for the file size and max file bytes
    pub fn new() -> Self {
        Self {
            max_block_size: u16::MAX,
            max_file_size: Self::DEFAULT_MAX_SIZE,
        }
    }
}

impl Default for FileReadConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for DirReadConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during a file operations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(feature = "ffi"), non_exhaustive)]
pub enum FileError {
    /// Outstation returned a bad response
    BadResponse,
    /// Outstation returned an error status code
    BadStatus(FileStatus),
    /// File handle returned did not match request
    WrongHandle,
    /// Outstation indicated no permission to access file
    NoPermission,
    /// Received an unexpected block number
    BadBlockNum,
    /// File transfer aborted by user
    AbortByUser,
    /// Exceeded the maximum length specified by the user
    MaxLengthExceeded,
    /// Generic task error occurred
    TaskError(TaskError),
}

impl From<tokio::sync::oneshot::error::RecvError> for FileError {
    fn from(_: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::TaskError(TaskError::Shutdown)
    }
}

impl From<Shutdown> for FileError {
    fn from(_: Shutdown) -> Self {
        Self::TaskError(TaskError::Shutdown)
    }
}

impl From<ObjectParseError> for FileError {
    fn from(value: ObjectParseError) -> Self {
        FileError::TaskError(value.into())
    }
}

impl From<SingleHeaderError> for FileError {
    fn from(_: SingleHeaderError) -> Self {
        Self::TaskError(TaskError::UnexpectedResponseHeaders)
    }
}

impl From<FileError> for TaskError {
    fn from(err: FileError) -> Self {
        match err {
            FileError::TaskError(err) => err,
            _ => TaskError::UnexpectedResponseHeaders,
        }
    }
}

impl From<TaskError> for FileError {
    fn from(err: TaskError) -> Self {
        FileError::TaskError(err)
    }
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FileError::BadResponse => f.write_str("bad response data"),
            FileError::BadStatus(s) => write!(f, "bad status code: {s:?}"),
            FileError::NoPermission => f.write_str("no permission"),
            FileError::BadBlockNum => f.write_str("bad block number"),
            FileError::AbortByUser => f.write_str("aborted by user"),
            FileError::TaskError(t) => Debug::fmt(&t, f),
            FileError::MaxLengthExceeded => f.write_str("exceeded maximum received length"),
            FileError::WrongHandle => {
                f.write_str("file handle returned by outstation did not match the request")
            }
        }
    }
}

/// Mode used when opening files
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FileMode {
    /// Code used for non-open command requests
    Null,
    /// Specifies that an existing file is to be opened for reading
    Read,
    /// Specifies that the file is to be opened for writing, truncating any existing file to length 0
    Write,
    /// Specifies that the file is to be opened for writing, appending to the end of the file
    Append,
    /// Used to capture reserved values
    Reserved(u16),
}

impl FileMode {
    pub(crate) fn new(value: u16) -> Self {
        match value {
            0 => Self::Null,
            1 => Self::Read,
            2 => Self::Write,
            3 => Self::Append,
            _ => Self::Reserved(value),
        }
    }

    pub(crate) fn to_u16(self) -> u16 {
        match self {
            Self::Null => 0,
            Self::Read => 1,
            Self::Write => 2,
            Self::Append => 3,
            Self::Reserved(x) => x,
        }
    }
}

impl std::error::Error for FileError {}

/// Describes whether a file operation should continue (No) or abort (Yes)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FileAction {
    /// Abort the operation
    Abort,
    /// Continue the operation
    Continue,
}

impl FileAction {
    pub(crate) fn is_abort(self) -> bool {
        match self {
            Self::Abort => true,
            Self::Continue => false,
        }
    }
}

/// Callbacks for reading a file
pub trait FileReader: Send + Sync + 'static {
    /// Called when the file is successfully opened
    ///
    /// May optionally abort the operation
    fn opened(&mut self, size: u32) -> FileAction;

    /// Called when the next block is received
    ///
    /// Returning ['FileAction::Abort'] will abort the transfer. This allows the application abort
    /// on internal errors like being or by user request.
    fn block_received(&mut self, block_num: u32, data: &[u8]) -> MaybeAsync<FileAction>;

    /// Called when the transfer is aborted before completion due to an error or user request
    fn aborted(&mut self, err: FileError);

    /// Called when the transfer completes successfully
    fn completed(&mut self);
}

/// Authentication key used when opening a file
#[derive(Copy, Clone, Debug)]
pub struct AuthKey(u32);

impl From<AuthKey> for u32 {
    fn from(value: AuthKey) -> Self {
        value.0
    }
}

impl AuthKey {
    /// Construct from a raw u32
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// The default authentication key (0)
    pub fn none() -> Self {
        Self(0)
    }
}

/// File handle assigned by the outstation
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FileHandle(u32);

impl From<FileHandle> for u32 {
    fn from(value: FileHandle) -> Self {
        value.0
    }
}

impl FileHandle {
    /// Construct from a raw value
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
}

/// Block number used in file read/write operations
#[derive(Copy, Clone, Debug, Default)]
pub struct BlockNumber(u32);

/// Error returned when a block cannot be incremented
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FailedBlockIncrement;

impl std::fmt::Display for FailedBlockIncrement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "File block number is at maximum value of {} and cannot be incremented",
            BlockNumber::MAX_VALUE
        )
    }
}

impl std::error::Error for FailedBlockIncrement {}

impl BlockNumber {
    const TOP_BIT: u32 = 0x80_00_00_00;

    /// Maximum possible block number
    pub const MAX_VALUE: u32 = !Self::TOP_BIT;

    /// Check if this is the last block
    pub fn is_last(self) -> bool {
        (self.0 & Self::TOP_BIT) != 0
    }

    /// Set the top bit to indicate this is the last block
    pub fn set_last(&mut self) {
        self.0 |= Self::TOP_BIT;
    }

    /// Try to increment the block number. If this fails, the maximum value
    /// is returned as an error.
    pub fn increment(&mut self) -> Result<(), FailedBlockIncrement> {
        let top_bit = self.0 & Self::TOP_BIT;
        let bottom_bits = self.bottom_bits();
        if bottom_bits < Self::MAX_VALUE {
            self.0 = (self.bottom_bits() + 1) | top_bit;
            Ok(())
        } else {
            Err(FailedBlockIncrement)
        }
    }

    /// Constructor used only in FFI mode
    #[cfg(feature = "ffi")]
    pub fn from_ffi_raw(raw: u32) -> BlockNumber {
        Self::new(raw)
    }

    pub(crate) fn new(raw: u32) -> BlockNumber {
        Self(raw)
    }

    pub(crate) fn bottom_bits(self) -> u32 {
        // The maximum value is also a mask for the bottom bits
        self.0 & Self::MAX_VALUE
    }

    pub(crate) fn wire_value(self) -> u32 {
        self.0
    }
}

/// The result of opening a file on the outstation
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OpenFile {
    /// The handle assigned to the file by the outstation
    ///
    /// This must be used in subsequent requests to manipulate the file
    pub file_handle: FileHandle,
    /// Size of the file returned by the outstation
    pub file_size: u32,
    /// Maximum block size returned by the outstation
    ///
    /// The master must respect this parameter when writing data to a file or
    /// the transfer may not succeed
    pub max_block_size: u16,
}
