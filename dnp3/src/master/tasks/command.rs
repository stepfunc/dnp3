use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::{HeaderCollection, Response};
use crate::app::FunctionCode;
use crate::master::error::{CommandError, CommandResponseError, TaskError};
use crate::master::promise::Promise;
use crate::master::request::*;
use crate::master::tasks::{AppTask, NonReadTask, Task};

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

    fn new(
        state: State,
        headers: CommandHeaders,
        promise: Promise<Result<(), CommandError>>,
    ) -> Self {
        Self {
            state,
            headers,
            promise,
        }
    }

    fn change_state(self, state: State) -> Self {
        Self::new(state, self.headers, self.promise)
    }

    pub(crate) fn wrap(self) -> NonReadTask {
        NonReadTask::Command(self)
    }

    pub(crate) fn function(&self) -> FunctionCode {
        match self.state {
            State::DirectOperate => FunctionCode::DirectOperate,
            State::Select => FunctionCode::Select,
            State::Operate => FunctionCode::Operate,
        }
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        self.headers.write(writer)
    }

    fn compare(&self, headers: HeaderCollection) -> Result<(), CommandResponseError> {
        self.headers.compare(headers)
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(err.into()))
    }

    pub(crate) fn handle(self, response: Response) -> Result<Option<NonReadTask>, TaskError> {
        let headers = match response.objects {
            Ok(x) => x,
            Err(err) => {
                self.promise
                    .complete(Err(TaskError::MalformedResponse(err).into()));
                return Err(TaskError::MalformedResponse(err));
            }
        };

        if let Err(err) = self.compare(headers) {
            self.promise.complete(Err(err.into()));
            return Err(TaskError::UnexpectedResponseHeaders);
        }

        match self.state {
            State::Select => Ok(Some(self.change_state(State::Operate).wrap())),
            _ => {
                // Complete w/ success
                self.promise.complete(Ok(()));
                Ok(None)
            }
        }
    }
}
