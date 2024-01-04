use crate::app::file::{FileStatus, FileType, Group70Var7, Permissions};
use crate::app::{FileMode, MaybeAsync, Shutdown, Timestamp};
use crate::master::{Promise, TaskError};
use scursor::ReadCursor;
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

/// Configuration related to reading a file
#[derive(Copy, Clone, Debug)]
pub struct FileWriteConfig {
    pub(crate) max_block_size: u16,
    pub(crate) mode: FileWriteMode,
    pub(crate) permissions: Permissions,
    pub(crate) file_size: u32,
}

impl FileWriteConfig {
    /// Construct with specified mode, permissions, and file size.
    ///
    /// The block size defaults to 1024
    ///
    /// `file_size` should be a non-zero value when opening a file for writing or appending.
    /// A file size of 0xFFFFFFFF indicates that the actual file size is unknown. Outstation
    /// devices are not required to accept unknown file sizes and may reject the request.
    pub fn new(mode: FileWriteMode, permissions: Permissions, file_size: u32) -> Self {
        Self {
            max_block_size: 1024,
            mode,
            permissions,
            file_size,
        }
    }

    /// Set the maximum block size requested by the master during the file open
    pub fn max_block_size(self, max_block_size: u16) -> Self {
        Self {
            max_block_size,
            ..self
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

impl From<Shutdown> for FileError {
    fn from(_: Shutdown) -> Self {
        Self::TaskError(TaskError::Shutdown)
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
            FileError::TaskError(t) => std::fmt::Debug::fmt(&t, f),
            FileError::MaxLengthExceeded => f.write_str("exceeded maximum received length"),
        }
    }
}

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

/// Block of file data
#[derive(Debug)]
pub struct Block {
    /// True if this was the final block of the file write operation
    pub last: bool,
    /// Bytes to be written to the destination buffer
    pub data: Vec<u8>,
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
    pub fn new(value: u32) -> Self {
        Self(value)
    }
}

/// Block number used in file read/write operations
#[derive(Copy, Clone, Debug, Default)]
pub struct BlockNumber(u32);

impl BlockNumber {
    const LAST_BIT: u32 = 0x80_00_00_00;

    /// Maximum possible block number
    pub const MAX_VALUE: u32 = !Self::LAST_BIT;

    /// Check if this is the last block
    pub fn is_last(self) -> bool {
        (self.0 & Self::LAST_BIT) != 0
    }

    pub(crate) fn new(raw: u32) -> BlockNumber {
        Self(raw)
    }

    pub(crate) fn set_last(&mut self) {
        self.0 |= Self::LAST_BIT;
    }

    pub(super) fn bottom_bits(self) -> u32 {
        // The maximum value is also a mask for the bottom bits
        self.0 & Self::MAX_VALUE
    }

    pub(crate) fn wire_value(self) -> u32 {
        self.0
    }

    pub(super) fn increment(&mut self) -> Result<(), u32> {
        if self.bottom_bits() < Self::MAX_VALUE {
            self.0 = self.bottom_bits() + 1;
            Ok(())
        } else {
            Err(Self::MAX_VALUE)
        }
    }
}

// TODO - see if this is really needed for correct behavior
impl PartialEq for BlockNumber {
    fn eq(&self, other: &Self) -> bool {
        let b1 = self.0 & Self::MAX_VALUE;
        let b2 = other.0 & Self::MAX_VALUE;
        b1 == b2
    }
}

/// The result of opening a file on the outstation
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OpenedFile {
    /// The handle assigned to the file by the outstation
    ///
    /// This should be used in subsequent requests to
    pub file_handle: FileHandle,
    /// Maximum block size returned by the outstation
    ///
    /// The master should respect this parameter when writing data to a file or
    /// the transfer may not succeed
    pub max_block_size: u16,
    /// Size of the file returned
    pub file_size: u32,
}

/// Callbacks for writing a file
pub trait FileWriter: Send + Sync + 'static {
    /// Called when the file is successfully opened by the outstation
    ///
    /// May optionally abort the operation
    fn opened(&mut self, handle: FileHandle, size: u32) -> FileAction;

    /// Called when the next block is needed for writing.
    ///
    /// The application will return Some(Block) upon success. Returning an empty block will terminate the transfer
    /// successfully and the master will then attempt to close the file. This can be useful if you do not know the
    /// file size or cannot detect the last block to be written.
    ///
    /// Returning None will fail the transfer. This allows the application to abort if there is an error
    /// reading the file or if the user wishes to terminate the transfer. The master will still attempt to close
    /// the file.
    fn next_block(&mut self, max_block_size: u16) -> MaybeAsync<Option<Block>>;

    /// Called when the transfer is aborted before completion due to an error or user request
    fn aborted(&mut self, err: FileError);

    /// Called when the transfer completes successfully
    fn completed(&mut self);
}

pub(crate) struct DirectoryReader {
    data: Vec<u8>,
    promise: Option<Promise<Result<Vec<FileInfo>, FileError>>>,
}

impl DirectoryReader {
    pub(crate) fn new(promise: Promise<Result<Vec<FileInfo>, FileError>>) -> Self {
        Self {
            data: Vec::new(),
            promise: Some(promise),
        }
    }
}

impl FileReader for DirectoryReader {
    fn opened(&mut self, _size: u32) -> FileAction {
        FileAction::Continue
    }

    fn block_received(&mut self, _block_num: u32, data: &[u8]) -> MaybeAsync<FileAction> {
        self.data.extend(data);
        MaybeAsync::ready(FileAction::Continue)
    }

    fn aborted(&mut self, err: FileError) {
        if let Some(x) = self.promise.take() {
            x.complete(Err(err));
        }
    }

    fn completed(&mut self) {
        fn parse(data: &[u8]) -> Result<Vec<FileInfo>, FileError> {
            let mut cursor = ReadCursor::new(data);
            let mut items = Vec::new();
            while !cursor.is_empty() {
                match Group70Var7::read(&mut cursor) {
                    Ok(x) => items.push(x),
                    Err(err) => {
                        tracing::warn!("Error reading directory information: {err}");
                        return Err(FileError::BadResponse);
                    }
                }
            }
            Ok(items.into_iter().map(|x| x.into()).collect())
        }

        // parse the accumulated data

        let res = parse(self.data.as_slice());
        if let Some(promise) = self.promise.take() {
            promise.complete(res);
        }
    }
}

impl<'a> From<Group70Var7<'a>> for FileInfo {
    fn from(value: Group70Var7<'a>) -> Self {
        Self {
            name: value.file_name.to_string(),
            file_type: value.file_type,
            size: value.file_size,
            time_created: value.time_of_creation,
            permissions: value.permissions,
        }
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for FileError {
    fn from(_: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::TaskError(TaskError::Shutdown)
    }
}
