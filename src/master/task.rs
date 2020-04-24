use crate::app::format::write::start_request;
use crate::app::gen::enums::FunctionCode;
use crate::app::header::{Control, ResponseHeader};
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::master::association::Association;
use crate::master::tasks::auto::AutoRequestDetails;
use crate::master::tasks::command::CommandTaskDetails;
use crate::master::runner::TaskError;
use crate::util::cursor::{WriteCursor, WriteError};

#[derive(Copy, Clone)]
pub(crate) enum TaskStatus {
    /// go through the whole cycle of formatting and waiting for a reply again
    ExecuteNextStep,
    /// The response was not for the task, so keep waiting on the current timeout
    ContinueWaiting,
    /// read another response with a new timeout
    ReadNextResponse,
    /// The task is complete
    Complete,
}

pub(crate) enum TaskDetails {
    // TODO - Read(ReadRequestDetails),
    Command(CommandTaskDetails),
    Auto(AutoRequestDetails),
}

impl TaskDetails {
    pub(crate) fn function(&self) -> FunctionCode {
        match self {
            // TODO - RequestDetails::Read(_) => FunctionCode::Read,
            TaskDetails::Command(x) => x.function(),
            TaskDetails::Auto(x) => x.function(),
        }
    }

    pub(crate) fn format(&self, seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        let mut writer = start_request(Control::request(seq), self.function(), cursor)?;
        match self {
            // TODO - RequestDetails::Read(task) => task.format(&mut writer),
            TaskDetails::Command(task) => task.format(&mut writer),
            TaskDetails::Auto(task) => task.format(&mut writer),
        }
    }

    pub(crate) fn handle(
        &mut self,
        session: &mut Association,
        _source: u16,
        response: ResponseHeader,
        headers: HeaderCollection,
    ) -> TaskStatus {
        match self {
            // TODO - RequestDetails::Read(task) => task.handle(source, response, headers),
            TaskDetails::Command(_task) => TaskStatus::Complete, // TODO - task.handle(headers),

            TaskDetails::Auto(task) => task.handle(session, response, headers),
        }
    }

    pub(crate) fn on_complete(&mut self, result: Result<(), TaskError>) {
        match self {
            TaskDetails::Auto(_) => {}
            // TODO - RequestDetails::Read(task) => task.on_complete(result),
            TaskDetails::Command(task) => task.on_complete(result),
        }
    }
}

pub(crate) struct Task {
    pub(crate) address: u16,
    pub(crate) details: TaskDetails,
}

impl Task {
    pub(crate) fn new(address: u16, details: TaskDetails) -> Self {
        Self { address, details }
    }
}
