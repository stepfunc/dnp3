use dnp3::master::*;
use std::time::Duration;

impl<T> sfio_promise::FutureType<Result<T, WriteError>> for crate::ffi::EmptyResponseCallback {
    fn on_drop() -> Result<T, WriteError> {
        Err(TaskError::Shutdown.into())
    }

    fn complete(self, result: Result<T, WriteError>) {
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
