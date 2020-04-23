use crate::app::format::write::HeaderWriter;
use crate::app::gen::enums::FunctionCode;
use crate::app::parse::parser::HeaderCollection;
use crate::master::request::{RequestDetails, RequestStatus};
use crate::master::runner::TaskError;
use crate::master::types::*;
use crate::util::cursor::WriteError;

enum State {
    Select,
    Operate,
    DirectOperate,
}

pub(crate) struct CommandRequestDetails {
    state: State,
    headers: Vec<CommandHeader>,
    handler: Box<dyn CommandTaskHandler>,
}

impl CommandRequestDetails {
    fn create(
        state: State,
        headers: Vec<CommandHeader>,
        handler: Box<dyn CommandTaskHandler>,
    ) -> RequestDetails {
        RequestDetails::Command(Self {
            state,
            headers,
            handler,
        })
    }

    pub(crate) fn select_before_operate(
        headers: Vec<CommandHeader>,
        handler: Box<dyn CommandTaskHandler>,
    ) -> RequestDetails {
        Self::create(State::Select, headers, handler)
    }

    pub(crate) fn direct_operate(
        headers: Vec<CommandHeader>,
        handler: Box<dyn CommandTaskHandler>,
    ) -> RequestDetails {
        Self::create(State::DirectOperate, headers, handler)
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

    pub(crate) fn handle(&mut self, headers: HeaderCollection) -> RequestStatus {
        if let Err(err) = self.compare(headers) {
            self.handler.on_command_complete(Err(err.into()));
            return RequestStatus::Complete;
        }

        match self.state {
            State::Select => {
                self.state = State::Operate;
                RequestStatus::ExecuteNextStep
            }
            _ => {
                // Complete w/ success
                self.handler.on_command_complete(Ok(()));
                RequestStatus::Complete
            }
        }
    }

    pub(crate) fn on_complete(&mut self, result: Result<(), TaskError>) {
        self.handler.on_complete(result);
    }
}
