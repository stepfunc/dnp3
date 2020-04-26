use crate::app::format::write::HeaderWriter;
use crate::app::gen::enums::FunctionCode;
use crate::app::parse::parser::{HeaderCollection, Response};
use crate::master::error::{CommandResponseError, TaskError};
use crate::master::handle::CommandCallback;
use crate::master::task::{NonReadTask, NonReadTaskStatus, TaskType};
use crate::master::types::*;
use crate::util::cursor::WriteError;

enum State {
    Select,
    Operate,
    DirectOperate,
}

pub(crate) struct CommandTask {
    state: State,
    headers: Vec<CommandHeader>,
    callback: CommandCallback,
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
    fn new(state: State, headers: Vec<CommandHeader>, callback: CommandCallback) -> Self {
        Self {
            state,
            headers,
            callback,
        }
    }

    fn change_state(self, state: State) -> NonReadTask {
        Self::get_non_read_task(state, self.headers, self.callback)
    }

    fn get_task_type(
        state: State,
        headers: Vec<CommandHeader>,
        callback: CommandCallback,
    ) -> TaskType {
        TaskType::NonRead(Self::get_non_read_task(state, headers, callback))
    }

    fn get_non_read_task(
        state: State,
        headers: Vec<CommandHeader>,
        callback: CommandCallback,
    ) -> NonReadTask {
        NonReadTask::Command(CommandTask::new(state, headers, callback))
    }

    pub(crate) fn operate(
        mode: CommandMode,
        headers: Vec<CommandHeader>,
        callback: CommandCallback,
    ) -> TaskType {
        Self::get_task_type(mode.to_state(), headers, callback)
    }

    pub(crate) fn function(&self) -> FunctionCode {
        match self.state {
            State::DirectOperate => FunctionCode::DirectOperate,
            State::Select => FunctionCode::Select,
            State::Operate => FunctionCode::Operate,
        }
    }

    pub(crate) fn format(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        for header in self.headers.iter() {
            header.write(writer)?;
        }

        Ok(())
    }

    fn compare(&self, headers: HeaderCollection) -> Result<(), CommandResponseError> {
        let mut iter = headers.iter();

        for sent in &self.headers {
            match iter.next() {
                None => return Err(CommandResponseError::HeaderCountMismatch),
                Some(received) => sent.compare(received.details)?,
            }
        }

        if iter.next().is_some() {
            return Err(CommandResponseError::HeaderCountMismatch);
        }

        Ok(())
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.callback.complete(Err(err.into()))
    }

    pub(crate) fn handle(self, response: Response) -> NonReadTaskStatus {
        let headers = match response.objects {
            Ok(x) => x,
            Err(err) => {
                self.callback
                    .complete(Err(TaskError::MalformedResponse(err).into()));
                return NonReadTaskStatus::Complete;
            }
        };

        if let Err(err) = self.compare(headers) {
            self.callback.complete(Err(err.into()));
            return NonReadTaskStatus::Complete;
        }

        match self.state {
            State::Select => NonReadTaskStatus::Next(self.change_state(State::Operate)),
            _ => {
                // Complete w/ success
                self.callback.complete(Ok(()));
                NonReadTaskStatus::Complete
            }
        }
    }
}
