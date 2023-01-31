use crate::app::file::{FileStatus, FileType, Group70Var7, Permissions};
use crate::app::Timestamp;
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
///
///
#[derive(Copy, Clone, Debug)]
pub struct FileReadConfig {
    pub(crate) max_block_size: u16,
    pub(crate) max_file_size: usize,
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

impl Default for FileReadConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during a file read operation
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(feature = "ffi"), non_exhaustive)]
pub enum FileReadError {
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
    TaskError(crate::master::TaskError),
}

impl std::fmt::Display for FileReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FileReadError::BadResponse => f.write_str("bad response data"),
            FileReadError::BadStatus(s) => write!(f, "bad status code: {s:?}"),
            FileReadError::NoPermission => f.write_str("no permission"),
            FileReadError::BadBlockNum => f.write_str("bad block number"),
            FileReadError::AbortByUser => f.write_str("aborted by user"),
            FileReadError::TaskError(t) => std::fmt::Debug::fmt(&t, f),
            FileReadError::MaxLengthExceeded => f.write_str("exceeded maximum received length"),
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
    fn block_received(&mut self, block_num: u32, data: &[u8]) -> bool;

    /// Called when the transfer is aborted before completion due to an error in the transfer
    fn aborted(&mut self, err: FileReadError);

    /// Called when the transfer completes
    fn completed(&mut self);
}

pub(crate) struct DirectoryReader {
    data: Vec<u8>,
}

impl DirectoryReader {
    pub(crate) fn new() -> Self {
        Self { data: Vec::new() }
    }
}

impl FileReader for DirectoryReader {
    fn opened(&mut self, _size: u32) -> bool {
        true
    }

    fn block_received(&mut self, _block_num: u32, data: &[u8]) -> bool {
        self.data.extend(data);
        true
    }

    fn aborted(&mut self, _err: FileReadError) {
        // complete the promise with an error
    }

    fn completed(&mut self) {
        fn parse(data: &[u8]) -> Result<Vec<FileInfo>, FileReadError> {
            let mut cursor = ReadCursor::new(data);
            let mut items = Vec::new();
            while !cursor.is_empty() {
                match Group70Var7::read(&mut cursor) {
                    Ok(x) => items.push(x),
                    Err(err) => {
                        tracing::warn!("Error reading directory information: {err}");
                        return Err(FileReadError::BadResponse);
                    }
                }
            }
            Ok(items.into_iter().map(|x| x.into()).collect())
        }

        // parse the accumulated data

        match parse(self.data.as_slice()) {
            Ok(items) => {
                // complete the promise!
                for item in items {
                    println!("File name: {}", item.name);
                    println!("  type: {:?}", item.file_type);
                    println!("  size: {}", item.size);
                    println!("  permissions:");
                    println!("     world: {}", item.permissions.world);
                    println!("     group: {}", item.permissions.group);
                    println!("     owner: {}", item.permissions.owner);
                }
            }
            Err(err) => {
                println!("error reading director: {err}");
            }
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
