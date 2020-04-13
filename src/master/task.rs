use crate::app::header::ResponseHeader;
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::master::handlers::ResponseHandler;
use crate::master::runner::TaskError;
use crate::master::tasks::command::CommandTask;
use crate::master::tasks::read::{ReadRequest, ReadTask};
use crate::master::types::CommandHeader;
use crate::util::cursor::{WriteCursor, WriteError};

#[derive(Copy, Clone, Debug)]
pub enum TaskStatus {
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
    Read(ReadTask),
    Command(CommandTask),
}

impl TaskDetails {
    pub(crate) fn is_read_request(&self) -> bool {
        match self {
            TaskDetails::Read(_) => true,
            TaskDetails::Command(_) => false,
        }
    }

    pub(crate) fn format(&self, seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        match self {
            TaskDetails::Read(task) => task.format(seq, cursor),
            TaskDetails::Command(task) => task.format(seq, cursor),
        }
    }

    pub(crate) fn handle(
        &mut self,
        source: u16,
        response: ResponseHeader,
        headers: HeaderCollection,
    ) -> TaskStatus {
        match self {
            TaskDetails::Read(task) => task.handle(source, response, headers),
            TaskDetails::Command(task) => task.handle(source, response, headers),
        }
    }

    pub(crate) fn on_error(&mut self, error: TaskError) {
        match self {
            TaskDetails::Read(task) => task.on_error(error),
            TaskDetails::Command(task) => task.on_error(error),
        }
    }
}

pub struct MasterTask {
    pub(crate) destination: u16,
    pub(crate) details: TaskDetails,
}

impl MasterTask {
    pub fn read(destination: u16, request: ReadRequest, handler: Box<dyn ResponseHandler>) -> Self {
        Self {
            destination,
            details: TaskDetails::Read(ReadTask { request, handler }),
        }
    }

    pub fn command(destination: u16, headers: Vec<CommandHeader>) -> Self {
        Self {
            destination,
            details: TaskDetails::Command(CommandTask::new(headers)),
        }
    }
}
