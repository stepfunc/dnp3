use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::{HeaderCollection, Response};
use crate::app::FunctionCode;
use crate::app::ResponseHeader;
use crate::link::EndpointAddress;
use crate::master::association::Association;
use crate::master::error::TaskError;
use crate::master::extract::extract_measurements;
use crate::master::handler::Promise;
use crate::master::poll::Poll;
use crate::master::request::{Classes, EventClasses};
use crate::master::tasks::auto::AutoTask;
use crate::master::tasks::command::CommandTask;
use crate::master::tasks::read::SingleReadTask;
use crate::master::tasks::restart::RestartTask;
use crate::master::tasks::time::TimeSyncTask;
use crate::master::{ReadType, TaskType};

use crate::master::tasks::deadbands::WriteDeadBandsTask;
use crate::master::tasks::empty_response::EmptyResponseTask;

pub(crate) mod auto;
pub(crate) mod command;
pub(crate) mod deadbands;
pub(crate) mod empty_response;
pub(crate) mod read;
pub(crate) mod restart;
pub(crate) mod time;

/// Queued task requiring I/O
pub(crate) struct AssociationTask {
    /// Outstation address
    pub(crate) address: EndpointAddress,
    /// Actual task to perform
    pub(crate) details: Task,
}

impl AssociationTask {
    pub(crate) fn new(address: EndpointAddress, details: Task) -> Self {
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
    /// Send link status request
    LinkStatus(Promise<Result<(), TaskError>>),
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) enum TaskId {
    LinkStatus,
    Function(FunctionCode),
}

impl Task {
    pub(crate) fn on_task_error(self, association: Option<&mut Association>, err: TaskError) {
        match self {
            Task::NonRead(task) => task.on_task_error(association, err),
            Task::Read(task) => task.on_task_error(association, err),
            Task::LinkStatus(promise) => promise.complete(Err(err)),
        }
    }

    /// Perform operation before sending and check if the request should still be sent
    ///
    /// Returning `true` means the task should proceed, returning false means
    /// the task was cancelled, forget about it.
    pub(crate) fn start(self, association: &mut Association) -> Option<Task> {
        if let Task::NonRead(task) = self {
            return task.start(association).map(|task| task.wrap());
        }

        Some(self)
    }

    pub(crate) fn get_id(&self) -> TaskId {
        match self {
            Task::LinkStatus(_) => TaskId::LinkStatus,
            Task::Read(_) => TaskId::Function(FunctionCode::Read),
            Task::NonRead(t) => TaskId::Function(t.function()),
        }
    }
}

pub(crate) trait RequestWriter {
    fn function(&self) -> FunctionCode;
    fn write(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError>;
}

pub(crate) enum ReadTask {
    /// Periodic polls that are configured when creating associations
    PeriodicPoll(Poll),
    /// Integrity poll that occurs during startup, or after outstation restarts
    StartupIntegrity(Classes),
    /// Event scan when IIN bit is detected
    EventScan(EventClasses),
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
    /// restart operation
    Restart(RestartTask),
    /// write dead-bands
    DeadBands(WriteDeadBandsTask),
    /// Generic task for anything that doesn't have response object headers
    EmptyResponseTask(EmptyResponseTask),
}

impl RequestWriter for ReadTask {
    fn function(&self) -> FunctionCode {
        FunctionCode::Read
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        match self {
            ReadTask::PeriodicPoll(poll) => poll.format(writer),
            ReadTask::StartupIntegrity(classes) => classes.write(writer),
            ReadTask::EventScan(classes) => classes.write(writer),
            ReadTask::SingleRead(req) => req.format(writer),
        }
    }
}

impl RequestWriter for NonReadTask {
    fn function(&self) -> FunctionCode {
        self.function()
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        match self {
            NonReadTask::Auto(t) => t.write(writer),
            NonReadTask::Command(t) => t.write(writer),
            NonReadTask::TimeSync(t) => t.write(writer),
            NonReadTask::Restart(_) => Ok(()),
            NonReadTask::DeadBands(t) => t.write(writer),
            NonReadTask::EmptyResponseTask(t) => t.write(writer),
        }
    }
}

impl ReadTask {
    pub(crate) fn wrap(self) -> Task {
        Task::Read(self)
    }

    pub(crate) async fn process_response(
        &mut self,
        association: &mut Association,
        header: ResponseHeader,
        objects: HeaderCollection<'_>,
    ) {
        match self {
            ReadTask::StartupIntegrity(_) => {
                association.handle_integrity_response(header, objects).await
            }
            ReadTask::PeriodicPoll(_) => association.handle_poll_response(header, objects).await,
            ReadTask::EventScan(_) => {
                association
                    .handle_event_scan_response(header, objects)
                    .await
            }
            ReadTask::SingleRead(task) => match &mut task.custom_handler {
                Some(handler) => {
                    extract_measurements(ReadType::SinglePoll, header, objects, handler.as_mut())
                        .await
                }
                None => association.handle_read_response(header, objects).await,
            },
        }
    }

    pub(crate) fn complete(self, association: &mut Association) {
        match self {
            ReadTask::StartupIntegrity(_) => association.on_integrity_scan_complete(),
            ReadTask::PeriodicPoll(poll) => association.complete_poll(poll.id),
            ReadTask::EventScan(_) => association.on_event_scan_complete(),
            ReadTask::SingleRead(task) => task.on_complete(),
        }
    }

    pub(crate) fn on_task_error(self, association: Option<&mut Association>, err: TaskError) {
        match self {
            ReadTask::StartupIntegrity(_) => {
                if let Some(association) = association {
                    association.on_integrity_scan_failure();
                }
            }
            ReadTask::PeriodicPoll(poll) => {
                if let Some(association) = association {
                    tracing::warn!("poll {} failed", poll.id);
                    association.complete_poll(poll.id);
                }
            }
            ReadTask::EventScan(_) => {
                if let Some(association) = association {
                    association.on_event_scan_failure();
                }
            }
            ReadTask::SingleRead(task) => task.on_task_error(err),
        }
    }

    pub(crate) fn as_task_type(&self) -> TaskType {
        match self {
            Self::PeriodicPoll(_) => TaskType::PeriodicPoll,
            Self::StartupIntegrity(_) => TaskType::StartupIntegrity,
            Self::EventScan(_) => TaskType::AutoEventScan,
            Self::SingleRead(_) => TaskType::UserRead,
        }
    }
}

impl NonReadTask {
    pub(crate) fn wrap(self) -> Task {
        Task::NonRead(self)
    }

    pub(crate) fn start(self, association: &mut Association) -> Option<NonReadTask> {
        match self {
            NonReadTask::Command(_) => Some(self),
            NonReadTask::Auto(_) => Some(self),
            NonReadTask::TimeSync(task) => task.start(association).map(|task| task.wrap()),
            NonReadTask::Restart(_) => Some(self),
            NonReadTask::DeadBands(_) => Some(self),
            NonReadTask::EmptyResponseTask(_) => Some(self),
        }
    }

    pub(crate) fn function(&self) -> FunctionCode {
        match self {
            NonReadTask::Command(task) => task.function(),
            NonReadTask::Auto(task) => task.function(),
            NonReadTask::TimeSync(task) => task.function(),
            NonReadTask::Restart(task) => task.function(),
            NonReadTask::DeadBands(task) => task.function(),
            NonReadTask::EmptyResponseTask(task) => task.function(),
        }
    }

    pub(crate) fn on_task_error(self, association: Option<&mut Association>, err: TaskError) {
        match self {
            NonReadTask::Command(task) => task.on_task_error(err),
            NonReadTask::TimeSync(task) => task.on_task_error(association, err),
            NonReadTask::Auto(task) => task.on_task_error(association, err),
            NonReadTask::Restart(task) => task.on_task_error(err),
            NonReadTask::DeadBands(task) => task.on_task_error(err),
            NonReadTask::EmptyResponseTask(task) => task.on_task_error(err),
        }
    }

    pub(crate) fn handle(
        self,
        association: &mut Association,
        response: Response,
    ) -> Option<NonReadTask> {
        match self {
            NonReadTask::Command(task) => task.handle(response),
            NonReadTask::Auto(task) => match response.objects.ok() {
                Some(headers) => task.handle(association, response.header, headers),
                None => None,
            },
            NonReadTask::TimeSync(task) => task.handle(association, response),
            NonReadTask::Restart(task) => task.handle(response),
            NonReadTask::DeadBands(task) => task.handle(response),
            NonReadTask::EmptyResponseTask(task) => task.handle(response),
        }
    }

    pub(crate) fn as_task_type(&self) -> TaskType {
        match self {
            Self::Command(_) => TaskType::Command,
            Self::Auto(x) => match x {
                AutoTask::ClearRestartBit => TaskType::ClearRestartBit,
                AutoTask::EnableUnsolicited(_) => TaskType::EnableUnsolicited,
                AutoTask::DisableUnsolicited(_) => TaskType::DisableUnsolicited,
            },
            Self::TimeSync(_) => TaskType::TimeSync,
            Self::Restart(_) => TaskType::Restart,
            NonReadTask::DeadBands(_) => TaskType::WriteDeadBands,
            NonReadTask::EmptyResponseTask(_) => TaskType::GenericEmptyResponse(self.function()),
        }
    }
}
