use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::parse::free_format::FreeFormatVariation;
use crate::app::parse::parser::{HeaderDetails, Response};
use crate::app::{FileStatus, FunctionCode, Group70Var4};
use crate::master::promise::Promise;
use crate::master::tasks::file::REQUEST_ID;
use crate::master::tasks::{AppTask, NonReadTask, Task};
use crate::master::{FileError, FileHandle, TaskError};

pub(crate) struct CloseFileTask {
    pub(crate) handle: FileHandle,
    pub(crate) promise: Promise<Result<(), FileError>>,
}

impl From<CloseFileTask> for Task {
    fn from(value: CloseFileTask) -> Self {
        Task::App(AppTask::NonRead(NonReadTask::CloseFile(value)))
    }
}

impl CloseFileTask {
    pub(crate) fn function(&self) -> FunctionCode {
        FunctionCode::CloseFile
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        let obj = Group70Var4 {
            file_handle: self.handle.into(),
            file_size: 0,
            max_block_size: 0,
            request_id: REQUEST_ID,
            status_code: FileStatus::Success,
            text: "",
        };

        writer.write_free_format(&obj)
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(err.into()))
    }

    pub(crate) fn handle(self, response: Response<'_>) -> Result<Option<NonReadTask>, TaskError> {
        fn process(expected_handle: FileHandle, response: Response<'_>) -> Result<(), FileError> {
            let header = response.objects?.get_only_header()?;

            let obj = match header.details {
                HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var4(obj)) => obj,
                _ => {
                    tracing::warn!(
                        "File CLOSE response contains unexpected variation: {}",
                        header.variation
                    );
                    return Err(FileError::BadResponse);
                }
            };

            if obj.file_handle != expected_handle.into() {
                return Err(FileError::WrongHandle);
            }

            if obj.status_code != FileStatus::Success {
                tracing::warn!(
                    "Unable to close file (status code == {:?})",
                    obj.status_code
                );
                return Err(FileError::BadStatus(obj.status_code));
            }

            Ok(())
        }

        let result = process(self.handle, response);
        self.promise.complete(result);

        match result {
            Ok(_) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
