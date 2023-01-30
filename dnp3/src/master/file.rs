use crate::app::file::FileStatus;

/// Credentials for obtaining a file authorization token from the outstation
#[derive(Clone, Debug)]
pub struct FileCredentials {
    /// User name
    pub user_name: String,
    /// Password
    pub password: String,
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
    /// Generic task error occurred
    TaskError(crate::master::TaskError),
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
