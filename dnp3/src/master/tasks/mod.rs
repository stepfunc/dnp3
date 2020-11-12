pub(crate) mod auto;
pub(crate) mod command;
pub(crate) mod read;
pub(crate) mod time;

use crate::app::enums::FunctionCode;
use crate::app::format::write::HeaderWriter;
use crate::app::header::ResponseHeader;
use crate::app::parse::parser::{HeaderCollection, Response};
use crate::master::association::Association;
use crate::master::error::TaskError;
use crate::master::poll::Poll;
use crate::master::tasks::auto::AutoTask;
use crate::master::tasks::command::CommandTask;
use crate::master::tasks::read::SingleReadTask;
use crate::master::tasks::time::TimeSyncTask;
use crate::util::cursor::WriteError;

/// Queued task requiring I/O
pub(crate) struct AssociationTask {
    /// Source address
    pub(crate) address: u16,
    /// Actual task to perform
    pub(crate) details: Task,
}

impl AssociationTask {
    pub(crate) fn new(address: u16, details: Task) -> Self {
        Self { address, details }
    }
}

/// There are two broad categories of tasks. Reads
/// require handling for multi-fragmented responses.
pub(crate) enum Task {
    /// Reads require handling for multi-fragmented responses
    Read(ReadTask),
    /// NonRead tasks always require FIR/FIN == 1, but might require multiple read/response cycles, e.g. SBO
    NonRead(NonReadTask),
}

impl Task {
    pub(crate) fn on_task_error(self, association: Option<&mut Association>, err: TaskError) {
        match self {
            Task::NonRead(task) => task.on_task_error(association, err),
            Task::Read(task) => task.on_task_error(association, err),
        }
    }
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
    /// One-time read request
    SingleRead(SingleReadTask),
}

pub(crate) enum NonReadTask {
    /// tasks that occur automatically during startup, or based on events or configuration,
    Auto(AutoTask),
    /// commands initiated from the user API
    Command(CommandTask),
    /// time synchronization
    TimeSync(TimeSyncTask),
}

impl RequestWriter for ReadTask {
    fn function(&self) -> FunctionCode {
        FunctionCode::Read
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match self {
            ReadTask::PeriodicPoll(poll) => poll.format(writer),
            ReadTask::StartupIntegrity => writer.write_class1230(),
            ReadTask::SingleRead(req) => req.format(writer),
        }
    }
}

impl RequestWriter for NonReadTask {
    fn function(&self) -> FunctionCode {
        self.function()
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match self {
            NonReadTask::Auto(t) => t.write(writer),
            NonReadTask::Command(t) => t.write(writer),
            NonReadTask::TimeSync(t) => t.write(writer),
        }
    }
}

impl ReadTask {
    pub(crate) fn wrap(self) -> Task {
        Task::Read(self)
    }

    pub(crate) fn process_response(
        &self,
        association: &mut Association,
        header: ResponseHeader,
        objects: HeaderCollection,
    ) {
        match self {
            ReadTask::StartupIntegrity => association.handle_integrity_response(header, objects),
            ReadTask::PeriodicPoll(_) => association.handle_poll_response(header, objects),
            ReadTask::SingleRead(_) => association.handle_read_response(header, objects),
        }
    }

    pub(crate) fn complete(self, association: &mut Association) {
        match self {
            ReadTask::StartupIntegrity => association.on_integrity_scan_complete(),
            ReadTask::PeriodicPoll(poll) => association.complete_poll(poll.id),
            ReadTask::SingleRead(task) => task.on_complete(),
        }
    }

    pub(crate) fn on_task_error(self, association: Option<&mut Association>, err: TaskError) {
        match self {
            ReadTask::StartupIntegrity => {
                if let Some(association) = association {
                    association.on_integrity_scan_failure();
                }
            }
            ReadTask::PeriodicPoll(_) => {}
            ReadTask::SingleRead(task) => task.on_task_error(err),
        }
    }
}

impl NonReadTask {
    pub(crate) fn wrap(self) -> Task {
        Task::NonRead(self)
    }

    pub(crate) fn function(&self) -> FunctionCode {
        match self {
            NonReadTask::Command(task) => task.function(),
            NonReadTask::Auto(task) => task.function(),
            NonReadTask::TimeSync(task) => task.function(),
        }
    }

    pub(crate) fn on_task_error(self, association: Option<&mut Association>, err: TaskError) {
        match self {
            NonReadTask::Command(task) => task.on_task_error(err),
            NonReadTask::TimeSync(task) => task.on_task_error(association, err),
            NonReadTask::Auto(task) => task.on_task_error(association, err),
        }
    }

    pub(crate) fn handle(
        self,
        request_tx: std::time::SystemTime,
        association: &mut Association,
        response: Response,
    ) -> Option<NonReadTask> {
        match self {
            NonReadTask::Command(task) => task.handle(response),
            NonReadTask::Auto(task) => match response.objects.ok() {
                Some(headers) => task.handle(association, response.header, headers),
                None => None,
            },
            NonReadTask::TimeSync(task) => task.handle(association, request_tx, response),
        }
    }
}
