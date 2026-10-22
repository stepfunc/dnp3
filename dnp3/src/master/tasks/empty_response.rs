use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::master::tasks::{NonReadTask, OneShot, OneShotRequest};
use crate::master::{Headers, TaskError, TaskType, WriteError};

/// Any request whose response has no object headers
pub(crate) struct EmptyResponseRequest {
    function: FunctionCode,
    headers: Headers,
}

impl EmptyResponseRequest {
    pub(crate) fn new(function: FunctionCode, headers: Headers) -> Self {
        Self { function, headers }
    }
}

/// Check that a response has no object headers
pub(crate) fn expect_empty(response: Response<'_>) -> Result<(), WriteError> {
    if response.raw_objects.is_empty() {
        Ok(())
    } else {
        Err(WriteError::Task(TaskError::UnexpectedResponseHeaders))
    }
}

pub(crate) fn write_error(err: WriteError) -> TaskError {
    match err {
        WriteError::Task(err) => err,
        #[expect(
            deprecated,
            reason = "the match must cover every variant until removal"
        )]
        WriteError::IinError(_) => TaskError::UnexpectedResponseHeaders,
    }
}

impl OneShotRequest for EmptyResponseRequest {
    type Output = ();
    type Error = WriteError;

    fn function(&self) -> FunctionCode {
        self.function
    }

    fn task_type(&self) -> TaskType {
        TaskType::GenericEmptyResponse(self.function)
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        self.headers.write(writer)
    }

    fn parse(&self, response: Response<'_>) -> Result<(), WriteError> {
        expect_empty(response)
    }

    fn task_error(err: WriteError) -> TaskError {
        write_error(err)
    }

    fn wrap(task: OneShot<Self>) -> NonReadTask {
        NonReadTask::EmptyResponseTask(task)
    }
}
