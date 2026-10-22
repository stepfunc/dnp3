use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::master::tasks::{NonReadTask, OneShot, OneShotRequest};
use crate::master::{FileError, FileHandle, TaskError, TaskType};

pub(crate) struct CloseFileRequest {
    pub(crate) handle: FileHandle,
}

impl OneShotRequest for CloseFileRequest {
    type Output = ();
    type Error = FileError;

    fn function(&self) -> FunctionCode {
        FunctionCode::CloseFile
    }

    fn task_type(&self) -> TaskType {
        TaskType::FileClose
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        Ok(super::write_close(self.handle, writer)?)
    }

    fn parse(&self, response: Response<'_>) -> Result<(), FileError> {
        let obj = super::parse_close_response(response.objects?.get_only_header()?)?;

        if obj.file_handle != u32::from(self.handle) {
            return Err(FileError::WrongHandle);
        }

        super::check_close_status(obj.status_code)
    }

    fn task_error(err: FileError) -> TaskError {
        err.into()
    }

    fn wrap(task: OneShot<Self>) -> NonReadTask {
        NonReadTask::CloseFile(task)
    }
}
