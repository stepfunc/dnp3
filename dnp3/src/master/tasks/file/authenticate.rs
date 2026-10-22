use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::master::promise::Promise;
use crate::master::tasks::{AppTask, NonReadTask, Task};
use crate::master::{AuthKey, FileCredentials, FileError, TaskError};

pub(crate) struct AuthFileTask {
    pub(crate) credentials: FileCredentials,
    pub(crate) promise: Promise<Result<AuthKey, FileError>>,
}

impl From<AuthFileTask> for Task {
    fn from(value: AuthFileTask) -> Self {
        Task::App(AppTask::NonRead(NonReadTask::AuthFile(value)))
    }
}

impl AuthFileTask {
    pub(crate) fn function(&self) -> FunctionCode {
        FunctionCode::AuthenticateFile
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        super::write_auth(&self.credentials, writer)
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(err.into()))
    }

    pub(crate) fn handle(self, response: Response<'_>) -> Result<Option<NonReadTask>, TaskError> {
        fn process(response: Response<'_>) -> Result<AuthKey, FileError> {
            super::parse_auth_response(response.objects?.get_only_header()?)
        }

        let result = process(response);
        self.promise.complete(result);

        match result {
            Ok(_) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
