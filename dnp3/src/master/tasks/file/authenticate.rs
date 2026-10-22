use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::master::tasks::{NonReadTask, OneShot, OneShotRequest};
use crate::master::{AuthKey, FileCredentials, FileError, TaskError, TaskType};

pub(crate) struct AuthFileRequest {
    pub(crate) credentials: FileCredentials,
}

impl OneShotRequest for AuthFileRequest {
    type Output = AuthKey;
    type Error = FileError;

    fn function(&self) -> FunctionCode {
        FunctionCode::AuthenticateFile
    }

    fn task_type(&self) -> TaskType {
        TaskType::FileAuth
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        Ok(super::write_auth(&self.credentials, writer)?)
    }

    fn parse(&self, response: Response<'_>) -> Result<AuthKey, FileError> {
        super::parse_auth_response(response.objects?.get_only_header()?)
    }

    fn wrap(task: OneShot<Self>) -> NonReadTask {
        NonReadTask::AuthFile(task)
    }
}
