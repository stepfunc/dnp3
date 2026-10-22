use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::parse::parser::Response;
use crate::app::{FunctionCode, Permissions};
use crate::master::promise::Promise;
use crate::master::tasks::{AppTask, NonReadTask, Task};
use crate::master::{AuthKey, FileError, FileMode, OpenFile, TaskError};

pub(crate) struct OpenFileRequest {
    pub(crate) file_name: String,
    pub(crate) auth_key: AuthKey,
    pub(crate) file_size: u32,
    pub(crate) file_mode: FileMode,
    pub(crate) permissions: Permissions,
    pub(crate) max_block_size: u16,
}

pub(crate) struct OpenFileTask {
    pub(crate) request: OpenFileRequest,
    pub(crate) promise: Promise<Result<OpenFile, FileError>>,
}

impl From<OpenFileTask> for Task {
    fn from(value: OpenFileTask) -> Self {
        Task::App(AppTask::NonRead(NonReadTask::OpenFile(value)))
    }
}

impl OpenFileTask {
    pub(crate) fn function(&self) -> FunctionCode {
        FunctionCode::OpenFile
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        super::write_open(
            &self.request.file_name,
            self.request.auth_key,
            self.request.permissions,
            self.request.file_size,
            self.request.file_mode,
            self.request.max_block_size,
            writer,
        )
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(err.into()))
    }

    pub(crate) fn handle(self, response: Response<'_>) -> Result<Option<NonReadTask>, TaskError> {
        fn process(response: Response<'_>) -> Result<OpenFile, FileError> {
            super::parse_open_response(response.objects?.get_only_header()?)
        }

        let result = process(response);
        self.promise.complete(result);

        match result {
            Ok(_) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
