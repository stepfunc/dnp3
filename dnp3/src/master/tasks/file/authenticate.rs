use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::parse::free_format::FreeFormatVariation;
use crate::app::parse::parser::{HeaderDetails, Response};
use crate::app::{FunctionCode, Group70Var2};
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
        let obj = Group70Var2 {
            auth_key: 0,
            user_name: &self.credentials.user_name,
            password: &self.credentials.password,
        };
        writer.write_free_format(&obj)
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(err.into()))
    }

    pub(crate) fn handle(self, response: Response<'_>) -> Result<Option<NonReadTask>, TaskError> {
        fn process(response: Response<'_>) -> Result<AuthKey, FileError> {
            let header = response.objects?.get_only_header()?;

            let obj = match header.details {
                HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var2(obj)) => obj,
                _ => {
                    tracing::warn!(
                        "File AUTHENTICATE response contains unexpected variation: {}",
                        header.variation
                    );
                    return Err(FileError::BadResponse);
                }
            };

            if obj.auth_key == 0 {
                tracing::warn!("Outstation returned auth key == 0: no permission to access file");
                return Err(FileError::NoPermission);
            }

            Ok(AuthKey::new(obj.auth_key))
        }

        let result = process(response);
        self.promise.complete(result);

        match result {
            Ok(_) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
