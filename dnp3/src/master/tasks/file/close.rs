use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::master::promise::Promise;
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
        super::write_close(self.handle, writer)
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(err.into()))
    }

    pub(crate) fn handle(self, response: Response<'_>) -> Result<Option<NonReadTask>, TaskError> {
        fn process(expected_handle: FileHandle, response: Response<'_>) -> Result<(), FileError> {
            let obj = super::parse_close_response(response.objects?.get_only_header()?)?;

            if obj.file_handle != u32::from(expected_handle) {
                return Err(FileError::WrongHandle);
            }

            super::check_close_status(obj.status_code)
        }

        let result = process(self.handle, response);
        self.promise.complete(result);

        match result {
            Ok(_) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
