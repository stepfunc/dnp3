use crate::app::format::write::HeaderWriter;
use crate::app::gen::enums::FunctionCode;
use crate::app::parse::parser::{HeaderCollection, Response};
use crate::master::error::{CommandResponseError, TaskError};
use crate::master::handle::{CommandResult, Promise};
use crate::master::task::{NonReadTask, TaskType};
use crate::master::types::*;
use crate::util::cursor::WriteError;

enum State {
    Select,
    Operate,
    DirectOperate,
}

pub(crate) struct CommandTask {
    state: State,
    headers: CommandHeaders,
    promise: Promise<CommandResult>,
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
    fn new(state: State, headers: CommandHeaders, promise: Promise<CommandResult>) -> Self {
        Self {
            state,
            headers,
            promise,
        }
    }

    fn change_state(self, state: State) -> NonReadTask {
        Self::get_non_read_task(state, self.headers, self.promise)
    }

    fn get_task_type(
        state: State,
        headers: CommandHeaders,
        promise: Promise<CommandResult>,
    ) -> TaskType {
        TaskType::NonRead(Self::get_non_read_task(state, headers, promise))
    }

    fn get_non_read_task(
        state: State,
        headers: CommandHeaders,
        promise: Promise<CommandResult>,
    ) -> NonReadTask {
        NonReadTask::Command(CommandTask::new(state, headers, promise))
    }

    pub(crate) fn operate(
        mode: CommandMode,
        headers: CommandHeaders,
        promise: Promise<CommandResult>,
    ) -> TaskType {
        Self::get_task_type(mode.to_state(), headers, promise)
    }

    pub(crate) fn function(&self) -> FunctionCode {
        match self.state {
            State::DirectOperate => FunctionCode::DirectOperate,
            State::Select => FunctionCode::Select,
            State::Operate => FunctionCode::Operate,
        }
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        self.headers.write(writer)
    }

    fn compare(&self, headers: HeaderCollection) -> Result<(), CommandResponseError> {
        self.headers.compare(headers)
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(err.into()))
    }

    pub(crate) fn handle(self, response: Response) -> Option<NonReadTask> {
        let headers = match response.objects {
            Ok(x) => x,
            Err(err) => {
                self.promise
                    .complete(Err(TaskError::MalformedResponse(err).into()));
                return None;
            }
        };

        if let Err(err) = self.compare(headers) {
            self.promise.complete(Err(err.into()));
            return None;
        }

        match self.state {
            State::Select => Some(self.change_state(State::Operate)),
            _ => {
                // Complete w/ success
                self.promise.complete(Ok(()));
                None
            }
        }
    }
}
