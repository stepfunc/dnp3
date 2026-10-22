use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::Response;
use crate::app::{FunctionCode, Permissions};
use crate::master::tasks::{NonReadTask, OneShot, OneShotRequest};
use crate::master::{AuthKey, FileError, FileMode, OpenFile, TaskError, TaskType};

pub(crate) struct OpenFileRequest {
    pub(crate) file_name: String,
    pub(crate) auth_key: AuthKey,
    pub(crate) file_size: u32,
    pub(crate) file_mode: FileMode,
    pub(crate) permissions: Permissions,
    pub(crate) max_block_size: u16,
}

impl OneShotRequest for OpenFileRequest {
    type Output = OpenFile;
    type Error = FileError;

    fn function(&self) -> FunctionCode {
        FunctionCode::OpenFile
    }

    fn task_type(&self) -> TaskType {
        TaskType::FileOpen
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        Ok(super::write_open(
            &self.file_name,
            self.auth_key,
            self.permissions,
            self.file_size,
            self.file_mode,
            self.max_block_size,
            writer,
        )?)
    }

    fn parse(&self, response: Response<'_>) -> Result<OpenFile, FileError> {
        super::parse_open_response(response.objects?.get_only_header()?)
    }

    fn wrap(task: OneShot<Self>) -> NonReadTask {
        NonReadTask::OpenFile(task)
    }
}
