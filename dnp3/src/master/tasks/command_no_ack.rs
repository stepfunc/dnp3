use crate::app::format::write::HeaderWriter;
use crate::app::FunctionCode;
use crate::master::error::TaskError;
use crate::master::promise::Promise;
use crate::master::request::CommandHeaders;
use crate::master::tasks::{AppTask, RequestWriter, Task};

/// Command sent with `DIRECT_OPERATE_NO_ACK`, which the outstation never responds to
pub(crate) struct CommandNoAckTask {
    headers: CommandHeaders,
    promise: Promise<Result<(), TaskError>>,
}

impl From<CommandNoAckTask> for Task {
    fn from(value: CommandNoAckTask) -> Self {
        Task::App(AppTask::NoResponse(value))
    }
}

impl RequestWriter for CommandNoAckTask {
    fn function(&self) -> FunctionCode {
        self.function()
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        self.headers.write(writer)
    }
}

impl CommandNoAckTask {
    pub(crate) fn new(headers: CommandHeaders, promise: Promise<Result<(), TaskError>>) -> Self {
        Self { headers, promise }
    }

    pub(crate) fn function(&self) -> FunctionCode {
        FunctionCode::DirectOperateNoResponse
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(err))
    }

    /// The request was written to the transport, which is all that can be known
    pub(crate) fn complete(self) {
        self.promise.complete(Ok(()))
    }
}
