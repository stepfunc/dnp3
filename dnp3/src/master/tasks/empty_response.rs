use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::master::promise::Promise;
use crate::master::tasks::{AppTask, NonReadTask, Task};
use crate::master::{Headers, TaskError, WriteError};

pub(crate) struct EmptyResponseTask {
    function: FunctionCode,
    headers: Headers,
    promise: Promise<Result<(), WriteError>>,
}

impl From<EmptyResponseTask> for Task {
    fn from(value: EmptyResponseTask) -> Self {
        Task::App(AppTask::NonRead(NonReadTask::EmptyResponseTask(value)))
    }
}

impl EmptyResponseTask {
    pub(crate) fn new(
        function: FunctionCode,
        headers: Headers,
        promise: Promise<Result<(), WriteError>>,
    ) -> Self {
        Self {
            function,
            headers,
            promise,
        }
    }

    pub(crate) const fn function(&self) -> FunctionCode {
        self.function
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        self.headers.write(writer)
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(err.into()))
    }

    pub(crate) fn handle(self, response: Response) -> Result<Option<NonReadTask>, TaskError> {
        if !response.raw_objects.is_empty() {
            self.promise
                .complete(Err(WriteError::Task(TaskError::UnexpectedResponseHeaders)));
            return Err(TaskError::UnexpectedResponseHeaders);
        }

        self.promise.complete(Ok(()));

        Ok(None)
    }
}
