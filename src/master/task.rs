use crate::app::format::write::HeaderWriter;
use crate::app::gen::enums::FunctionCode;
use crate::app::parse::parser::Response;
use crate::master::association::Association;
use crate::master::poll::Poll;
use crate::master::runner::TaskError;
use crate::master::tasks::auto::AutoTask;
use crate::master::tasks::command::CommandTask;
use crate::util::cursor::WriteError;

pub(crate) enum NonReadTaskStatus {
    /// The task is complete
    Complete,
    /// Another task follows
    Next(NonReadTask),
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

pub(crate) trait RequestWriter {
    fn function(&self) -> FunctionCode;
    fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError>;
}

pub(crate) enum ReadTask {
    /// Periodic polls that are configured when creating associations
    PeriodicPoll(Poll),
    /// Integrity poll that occurs during startup, or after outstation restarts
    StartupIntegrity,
}

pub(crate) enum NonReadTask {
    /// tasks that occur automatically during startup, or based on events or configuration,
    Auto(AutoTask),
    /// commands initiated from the user API
    Command(CommandTask),
}

impl RequestWriter for ReadTask {
    fn function(&self) -> FunctionCode {
        FunctionCode::Read
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match self {
            ReadTask::PeriodicPoll(poll) => poll.format(writer),
            ReadTask::StartupIntegrity => writer.write_class1230(),
        }
    }
}

impl RequestWriter for NonReadTask {
    fn function(&self) -> FunctionCode {
        self.function()
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match self {
            NonReadTask::Auto(t) => t.format(writer),
            NonReadTask::Command(t) => t.format(writer),
        }
    }
}

impl ReadTask {
    pub(crate) fn complete(self, association: &mut Association) {
        match self {
            ReadTask::StartupIntegrity => association.on_integrity_scan_complete(),
            ReadTask::PeriodicPoll(poll) => association.complete_poll(poll.id),
        }
    }
}

impl NonReadTask {
    pub(crate) fn function(&self) -> FunctionCode {
        match self {
            NonReadTask::Command(task) => task.function(),
            NonReadTask::Auto(task) => task.function(),
        }
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        match self {
            NonReadTask::Command(task) => task.on_task_error(err),
            NonReadTask::Auto(_) => {}
        }
    }

    pub(crate) fn handle(
        self,
        association: &mut Association,
        response: Response,
    ) -> NonReadTaskStatus {
        match self {
            NonReadTask::Command(task) => task.handle(response),
            NonReadTask::Auto(task) => match response.objects.ok() {
                Some(headers) => task.handle(association, response.header, headers),
                None => NonReadTaskStatus::Complete,
            },
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
