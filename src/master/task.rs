use crate::app::format::write::start_request;
use crate::app::gen::enums::FunctionCode;
use crate::app::header::{Control, ResponseHeader};
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::master::association::Association;
use crate::master::runner::TaskError;
use crate::master::tasks::auto::AutoTask;
use crate::master::tasks::command::CommandTask;
use crate::util::cursor::{WriteCursor, WriteError};
use std::process::Command;

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

/// There are two broad categories of tasks. Reads
/// require handling for multi-fragmented responses.
///
pub(crate) enum TaskType {
    /// Reads require handling for multi-fragmented responses
    Read(ReadTask),
    /// NonRead tasks always require FIR/FIN == 1, but might require multiple read/response cycles, e.g. SBO
    NonRead(NonReadTask),
}

pub(crate) enum ReadTask {
    /// Periodic polls that are configured when creating associations
    PeriodicPoll,
    /// Integrity poll that occurs during startup, or after outstation restarts
    StartupIntegrity,
}

pub(crate) enum NonReadTask {
    /// tasks that occur automatically during startup, or based on events or configuration,
    Auto(AutoTask),
    /// commands initiated from the user API
    Command(CommandTask),
}

impl TaskType {
    // this method just gets used for formatting the request
    /*
    pub(crate) fn function(&self) -> FunctionCode {
        match self {
            TaskType::Read(_) => FunctionCode::Read,
            TaskType::NonRead(task) => task.function(),
        }
    }

    pub(crate) fn format(&self, seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        let mut writer = start_request(Control::request(seq), self.function(), cursor)?;
        match self {
            TaskType::Read(task) => task.format(&mut writer),
            TaskType::NonRead(task) => task.format(&mut writer),
        }
    }
    */
}

impl NonReadTask {
    pub(crate) fn command(task: CommandTask) -> TaskType {
        TaskType::NonRead(NonReadTask::Command(task))
    }

    pub(crate) fn auto(task: AutoTask) -> TaskType {
        TaskType::NonRead(NonReadTask::Auto(task))
    }

    pub(crate) fn function(&self) -> FunctionCode {
        match self {
            NonReadTask::Command(task) => task.function(),
            NonReadTask::Auto(task) => task.function(),
        }
    }
}

pub(crate) struct Task {
    pub(crate) address: u16,
    pub(crate) details: TaskType,
}

impl Task {
    pub(crate) fn new(address: u16, details: TaskType) -> Self {
        Self { address, details }
    }
}
