use crate::app::file::{FileStatus, FileType, Group70Var7, Permissions};
use crate::app::{MaybeAsync, Shutdown, Timestamp};
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

/// Represents the status of a file operation in progress
pub struct FileOperation {
    canceler: crate::util::cancelation::Canceler,
    reply: tokio::sync::oneshot::Receiver<Result<(), FileError>>,
}

impl FileOperation {
    pub(crate) fn new(
        canceler: crate::util::cancelation::Canceler,
        reply: tokio::sync::oneshot::Receiver<Result<(), FileError>>,
    ) -> Self {
        Self { canceler, reply }
    }

    /// request that the file operation aborts at the next opportunity
    pub async fn abort(&self) {
        self.canceler.cancel();
    }

    /// await the result of the file operation
    pub async fn result(self) -> Result<(), FileError> {
        self.reply.await?
    }
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
///
///
#[derive(Copy, Clone, Debug)]
pub struct FileReadConfig {
    pub(crate) max_block_size: u16,
    pub(crate) max_file_size: usize,
}

/// Configuration related to reading a directory
///
///
#[derive(Copy, Clone, Debug)]
pub struct DirReadConfig {
    pub(crate) max_block_size: u16,
    pub(crate) max_file_size: usize,
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

    /// Modify the maximum block size requested by the master during the file open
    pub fn set_max_block_size(self, max_block_size: u16) -> Self {
        Self {
            max_block_size,
            ..self
        }
    }

    /// Modify the maximum file size accepted by the master
    ///
    /// This applies to both:
    ///
    /// 1) The size returned by the outstation in the OPEN operation
    /// 2) The total number of bytes actually returned in subsequent READ operations
    pub fn set_max_file_size(self, max_file_size: usize) -> Self {
        Self {
            max_file_size,
            ..self
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

    /// Modify the maximum block size requested by the master during the file open
    pub fn set_max_block_size(self, max_block_size: u16) -> Self {
        Self {
            max_block_size,
            ..self
        }
    }

    /// Modify the maximum number of bytes that may be accumulated while reading
    /// directory information
    pub fn set_max_size(self, max_size: usize) -> Self {
        Self {
            max_file_size: max_size,
            ..self
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

/// Callbacks for reading a file
pub trait FileReader: Send + Sync + 'static {
    /// Called when the file is successfully opened
    ///
    /// Returning false will abort the transfer
    fn opened(&mut self, size: u32) -> bool;

    /// Called when the next block is received
    ///
    /// Returning false will abort the transfer. This allows the received to place
    /// limits on the amount of received data or abort on internals errors like being
    /// unable to write to a local file
    fn block_received(&mut self, block_num: u32, data: &[u8]) -> MaybeAsync<bool>;

    /// Called when the transfer is aborted before completion due to an error in the transfer
    fn aborted(&mut self, err: FileError);

    /// Called when the transfer completes
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
    fn opened(&mut self, _size: u32) -> bool {
        true
    }

    fn block_received(&mut self, _block_num: u32, data: &[u8]) -> MaybeAsync<bool> {
        self.data.extend(data);
        MaybeAsync::ready(true)
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

impl From<Shutdown> for FileError {
    fn from(_: Shutdown) -> Self {
        Self::TaskError(TaskError::Shutdown)
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for FileError {
    fn from(_: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::TaskError(TaskError::Shutdown)
    }
}
