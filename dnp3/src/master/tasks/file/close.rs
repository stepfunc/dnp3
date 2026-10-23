// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

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
        super::check_close_response(self.handle, response.objects?.get_only_header()?)
    }

    fn wrap(task: OneShot<Self>) -> NonReadTask {
        NonReadTask::CloseFile(task)
    }
}
