use crate::app::format::write::HeaderWriter;
use crate::app::gen::enums::FunctionCode;
use crate::app::parse::parser::HeaderCollection;
use crate::master::handlers::CommandCallback;
use crate::master::runner::{CommandMode, TaskError};
use crate::master::task::{NonReadTask, TaskStatus, TaskType};
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
    fn create(state: State, headers: Vec<CommandHeader>, callback: CommandCallback) -> TaskType {
        NonReadTask::command(Self {
            state,
            headers,
            callback,
        })
    }

    pub(crate) fn operate(
        mode: CommandMode,
        headers: Vec<CommandHeader>,
        callback: CommandCallback,
    ) -> TaskType {
        Self::create(mode.to_state(), headers, callback)
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

    pub(crate) fn handle(&mut self, _headers: HeaderCollection) -> TaskStatus {
        TaskStatus::Complete
        /*
        if let Err(err) = self.compare(headers) {
            self.callback.complete(Err(err.into()));
            return RequestStatus::Complete;
        }

        match self.state {
            State::Select => {
                self.state = State::Operate;
                RequestStatus::ExecuteNextStep
            }
            _ => {
                // Complete w/ success
                self.callback.complete(Ok(()));
                RequestStatus::Complete
            }
        }
        */
    }

    pub(crate) fn on_complete(&mut self, _result: Result<(), TaskError>) {
        // self.callback.complete(result); // TODO
    }
}
