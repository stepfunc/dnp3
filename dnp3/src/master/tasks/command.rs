use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::master::association::Association;
use crate::master::error::{CommandError, TaskError};
use crate::master::promise::Promise;
use crate::master::request::*;
use crate::master::tasks::{AppTask, NonReadTask, ResponseTask, Step, Task};
use crate::master::TaskType;

enum State {
    Select,
    Operate,
    DirectOperate,
}

pub(crate) struct CommandTask {
    state: State,
    headers: CommandHeaders,
    promise: Promise<Result<(), CommandError>>,
}

impl From<CommandTask> for Task {
    fn from(value: CommandTask) -> Self {
        Task::App(AppTask::NonRead(NonReadTask::Command(value)))
    }
}

impl CommandMode {
    fn to_state(self) -> State {
        match self {
            CommandMode::DirectOperate => State::DirectOperate,
            CommandMode::SelectBeforeOperate => State::Select,
        }
    }
}

impl CommandTask {
    pub(crate) fn from_mode(
        mode: CommandMode,
        headers: CommandHeaders,
        promise: Promise<Result<(), CommandError>>,
    ) -> Self {
        Self {
            state: mode.to_state(),
            headers,
            promise,
        }
    }
}

impl ResponseTask for CommandTask {
    type Output = ();
    type Error = CommandError;

    fn function(&self) -> FunctionCode {
        match self.state {
            State::DirectOperate => FunctionCode::DirectOperate,
            State::Select => FunctionCode::Select,
            State::Operate => FunctionCode::Operate,
        }
    }

    fn task_type(&self) -> TaskType {
        TaskType::Command
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        self.headers.write(writer)
    }

    async fn on_response(
        &mut self,
        _association: &mut Association,
        response: Response<'_>,
    ) -> Result<Step<()>, CommandError> {
        let headers = response.objects.map_err(TaskError::MalformedResponse)?;

        self.headers.compare(headers)?;

        match self.state {
            State::Select => {
                self.state = State::Operate;
                Ok(Step::Next)
            }
            State::Operate | State::DirectOperate => Ok(Step::Done(())),
        }
    }

    fn complete(self, _association: Option<&mut Association>, result: Result<(), CommandError>) {
        self.promise.complete(result)
    }
}
