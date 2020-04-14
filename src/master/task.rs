use crate::app::format::write::start_request;
use crate::app::gen::enums::FunctionCode;
use crate::app::header::{Control, ResponseHeader};
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::master::handlers::{ReadTaskHandler, TaskCompletionHandler};
use crate::master::runner::TaskError;
use crate::master::tasks::command::CommandTask;
use crate::master::tasks::read::ReadTask;
use crate::master::tasks::simple::BasicResponseTask;
use crate::master::types::{
    BasicRequest, CommandHeader, CommandTaskHandler, EventClasses, ReadRequest,
};
use crate::util::cursor::{WriteCursor, WriteError};

#[derive(Copy, Clone, Debug)]
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
    Read(ReadTask),
    Command(CommandTask),
    BasicRequest(BasicResponseTask),
}

impl TaskDetails {
    pub(crate) fn function(&self) -> FunctionCode {
        match self {
            TaskDetails::Read(_) => FunctionCode::Read,
            TaskDetails::Command(x) => x.function(),
            TaskDetails::BasicRequest(x) => x.function(),
        }
    }

    pub(crate) fn format(&self, seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        let mut writer = start_request(Control::request(seq), self.function(), cursor)?;
        match self {
            TaskDetails::Read(task) => task.format(&mut writer),
            TaskDetails::Command(task) => task.format(&mut writer),
            TaskDetails::BasicRequest(task) => task.format(&mut writer),
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
            TaskDetails::BasicRequest(task) => task.handle(source, response, headers),
        }
    }

    pub(crate) fn on_complete(&mut self, result: Result<(), TaskError>) {
        match self {
            TaskDetails::Read(task) => task.on_complete(result),
            TaskDetails::Command(task) => task.on_complete(result),
            TaskDetails::BasicRequest(task) => task.on_complete(result),
        }
    }
}

pub struct MasterTask {
    pub(crate) destination: u16,
    pub(crate) details: TaskDetails,
}

impl MasterTask {
    pub fn read(destination: u16, request: ReadRequest, handler: Box<dyn ReadTaskHandler>) -> Self {
        Self {
            destination,
            details: TaskDetails::Read(ReadTask { request, handler }),
        }
    }

    pub fn disable_unsolicited(
        destination: u16,
        classes: EventClasses,
        handler: Box<dyn TaskCompletionHandler>,
    ) -> Self {
        Self {
            destination,
            details: TaskDetails::BasicRequest(BasicResponseTask {
                request: BasicRequest::DisableUnsolicited(classes),
                handler,
            }),
        }
    }

    pub fn enable_unsolicited(
        destination: u16,
        classes: EventClasses,
        handler: Box<dyn TaskCompletionHandler>,
    ) -> Self {
        Self {
            destination,
            details: TaskDetails::BasicRequest(BasicResponseTask {
                request: BasicRequest::EnableUnsolicited(classes),
                handler,
            }),
        }
    }

    pub fn select_before_operate(
        destination: u16,
        headers: Vec<CommandHeader>,
        handler: Box<dyn CommandTaskHandler>,
    ) -> Self {
        Self {
            destination,
            details: TaskDetails::Command(CommandTask::select_before_operate(headers, handler)),
        }
    }

    pub fn direct_operate(
        destination: u16,
        headers: Vec<CommandHeader>,
        handler: Box<dyn CommandTaskHandler>,
    ) -> Self {
        Self {
            destination,
            details: TaskDetails::Command(CommandTask::direct_operate(headers, handler)),
        }
    }
}
