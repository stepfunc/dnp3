use crate::{ffi, FileInfoIterator};
use dnp3::master::*;
use std::ffi::CString;
use std::time::Duration;

impl<T> sfio_promise::FutureType<Result<T, WriteError>> for ffi::EmptyResponseCallback {
    fn on_drop() -> Result<T, WriteError> {
        Err(TaskError::Shutdown.into())
    }

    fn complete(self, result: Result<T, WriteError>) {
        match result {
            Ok(_) => {
                self.on_complete(ffi::Nothing::Nothing);
            }
            Err(err) => {
                self.on_failure(err.into());
            }
        }
    }
}

impl sfio_promise::FutureType<Result<AuthKey, FileError>> for ffi::FileAuthCallback {
    fn on_drop() -> Result<AuthKey, FileError> {
        Err(TaskError::Shutdown.into())
    }

    fn complete(self, result: Result<AuthKey, FileError>) {
        match result {
            Ok(x) => self.on_complete(x.into()),
            Err(err) => self.on_failure(err.into()),
        }
    }
}

impl sfio_promise::FutureType<Result<OpenFile, FileError>> for ffi::FileOpenCallback {
    fn on_drop() -> Result<OpenFile, FileError> {
        Err(TaskError::Shutdown.into())
    }

    fn complete(self, result: Result<OpenFile, FileError>) {
        match result {
            Ok(x) => self.on_complete(x.into()),
            Err(err) => self.on_failure(err.into()),
        }
    }
}

impl sfio_promise::FutureType<Result<(), FileError>> for ffi::FileOperationCallback {
    fn on_drop() -> Result<(), FileError> {
        Err(TaskError::Shutdown.into())
    }

    fn complete(self, result: Result<(), FileError>) {
        match result {
            Ok(()) => self.on_complete(ffi::Nothing::Nothing),
            Err(err) => self.on_failure(err.into()),
        }
    }
}

impl sfio_promise::FutureType<Result<(), TaskError>> for crate::ffi::ReadTaskCallback {
    fn on_drop() -> Result<(), TaskError> {
        Err(TaskError::Shutdown)
    }

    fn complete(self, result: Result<(), TaskError>) {
        match result {
            Ok(_) => {
                self.on_complete(crate::ffi::Nothing::Nothing);
            }
            Err(err) => {
                self.on_failure(err.into());
            }
        }
    }
}

impl sfio_promise::FutureType<Result<(), CommandError>> for crate::ffi::CommandTaskCallback {
    fn on_drop() -> Result<(), CommandError> {
        Err(TaskError::Shutdown.into())
    }

    fn complete(self, result: Result<(), CommandError>) {
        match result {
            Ok(_) => {
                self.on_complete(crate::ffi::Nothing::Nothing);
            }
            Err(err) => {
                self.on_failure(err.into());
            }
        }
    }
}

impl sfio_promise::FutureType<Result<(), TimeSyncError>> for crate::ffi::TimeSyncTaskCallback {
    fn on_drop() -> Result<(), TimeSyncError> {
        Err(TaskError::Shutdown.into())
    }

    fn complete(self, result: Result<(), TimeSyncError>) {
        match result {
            Ok(_) => {
                self.on_complete(crate::ffi::Nothing::Nothing);
            }
            Err(err) => {
                self.on_failure(err.into());
            }
        }
    }
}

impl sfio_promise::FutureType<Result<Duration, TaskError>> for crate::ffi::RestartTaskCallback {
    fn on_drop() -> Result<Duration, TaskError> {
        Err(TaskError::Shutdown)
    }

    fn complete(self, result: Result<Duration, TaskError>) {
        match result {
            Ok(x) => {
                self.on_complete(x);
            }
            Err(err) => {
                self.on_failure(err.into());
            }
        }
    }
}

impl sfio_promise::FutureType<Result<(), TaskError>> for crate::ffi::LinkStatusCallback {
    fn on_drop() -> Result<(), TaskError> {
        Err(TaskError::Shutdown)
    }

    fn complete(self, result: Result<(), TaskError>) {
        match result {
            Ok(()) => {
                self.on_complete(crate::ffi::Nothing::Nothing);
            }
            Err(err) => {
                self.on_failure(err.into());
            }
        }
    }
}

impl sfio_promise::FutureType<Result<FileInfo, FileError>> for crate::ffi::FileInfoCallback {
    fn on_drop() -> Result<FileInfo, FileError> {
        Err(FileError::TaskError(TaskError::Shutdown))
    }

    fn complete(self, result: Result<FileInfo, FileError>) {
        match result {
            Ok(info) => {
                // this stays validate for the duration of the completion callback allowing
                // the C string to be copied
                let name = CString::new(info.name).unwrap();
                let info = ffi::FileInfoFields {
                    file_name: &name,
                    file_type: info.file_type.into(),
                    size: info.size,
                    time_created: info.time_created.raw_value(),
                    permissions: info.permissions.into(),
                };
                self.on_complete(info.into());
            }
            Err(err) => {
                self.on_failure(err.into());
            }
        }
    }
}

impl sfio_promise::FutureType<Result<Vec<FileInfo>, FileError>>
    for crate::ffi::ReadDirectoryCallback
{
    fn on_drop() -> Result<Vec<FileInfo>, FileError> {
        Err(FileError::TaskError(TaskError::Shutdown))
    }

    fn complete(self, result: Result<Vec<FileInfo>, FileError>) {
        match result {
            Ok(items) => {
                let mut iter = FileInfoIterator::new(items.into_iter());
                self.on_complete(&mut iter);
            }
            Err(err) => {
                self.on_failure(err.into());
            }
        }
    }
}

impl From<dnp3::app::FileType> for ffi::FileType {
    fn from(value: dnp3::app::FileType) -> Self {
        match value {
            dnp3::app::FileType::Directory => Self::Directory,
            dnp3::app::FileType::File => Self::Simple,
            dnp3::app::FileType::Other(_) => Self::Other,
        }
    }
}

impl From<dnp3::app::Permissions> for ffi::Permissions {
    fn from(value: dnp3::app::Permissions) -> Self {
        Self {
            world: value.world.into(),
            group: value.world.into(),
            owner: value.group.into(),
        }
    }
}

impl From<dnp3::app::PermissionSet> for ffi::PermissionSet {
    fn from(value: dnp3::app::PermissionSet) -> Self {
        Self {
            execute: value.execute,
            write: value.write,
            read: value.read,
        }
    }
}

impl From<FileError> for ffi::FileError {
    fn from(value: FileError) -> Self {
        match value {
            FileError::BadResponse => ffi::FileError::BadResponse,
            FileError::BadStatus(_) => ffi::FileError::BadStatus,
            FileError::NoPermission => ffi::FileError::NoPermission,
            FileError::BadBlockNum => ffi::FileError::BadBlockNum,
            FileError::AbortByUser => ffi::FileError::AbortByUser,
            FileError::MaxLengthExceeded => ffi::FileError::MaxLengthExceeded,
            FileError::TaskError(x) => x.into(),
            FileError::WrongHandle => ffi::FileError::WrongHandle,
        }
    }
}

impl From<OpenFile> for ffi::OpenFile {
    fn from(value: OpenFile) -> Self {
        Self {
            file_handle: value.file_handle.into(),
            file_size: value.file_size,
            max_block_size: value.max_block_size,
        }
    }
}
