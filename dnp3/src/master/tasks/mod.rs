use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::{HeaderCollection, Response};
use crate::app::FunctionCode;
use crate::app::ResponseHeader;
use crate::master::association::Association;
use crate::master::error::TaskError;
use crate::master::extract::extract_measurements;
use crate::master::poll::Poll;
use crate::master::promise::Promise;
use crate::master::request::{Classes, EventClasses};
use crate::master::tasks::auto::AutoTask;
use crate::master::tasks::command::CommandTask;
use crate::master::tasks::read::SingleReadTask;
use crate::master::tasks::restart::RestartTask;
use crate::master::tasks::time::TimeSyncTask;
use crate::master::{ReadType, TaskType};

use crate::master::tasks::deadbands::WriteDeadBandsTask;
use crate::master::tasks::empty_response::EmptyResponseTask;
use crate::master::tasks::file::authenticate::AuthFileTask;
use crate::master::tasks::file::close::CloseFileTask;
use crate::master::tasks::file::get_info::GetFileInfoTask;
use crate::master::tasks::file::open::OpenFileTask;
use crate::master::tasks::file::read::FileReadTask;
use crate::master::tasks::file::write_block::WriteBlockTask;
use crate::transport::FragmentAddr;

pub(crate) mod auto;
pub(crate) mod command;
pub(crate) mod deadbands;
pub(crate) mod empty_response;

pub(crate) mod file;
pub(crate) mod read;
pub(crate) mod restart;
pub(crate) mod time;

/// Queued task requiring I/O
pub(crate) struct AssociationTask {
    /// Destination addresses for tasks
    pub(crate) dest: FragmentAddr,
    /// Actual task to perform
    pub(crate) details: Task,
}

impl AssociationTask {
    pub(crate) fn new(dest: FragmentAddr, details: Task) -> Self {
        Self { dest, details }
    }
}

/// There are two broad categories of tasks. Reads
/// require handling for multi-fragmented responses.
pub(crate) enum AppTask {
    /// Reads require handling for multi-fragmented responses
    Read(ReadTask),
    /// NonRead tasks always require FIR/FIN == 1, but might require multiple read/response cycles, e.g. SBO
    NonRead(NonReadTask),
}

pub(crate) enum Task {
    /// An application layer task
    App(AppTask),
    /// Send link status request
    LinkStatus(Promise<Result<(), TaskError>>),
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) enum TaskId {
    LinkStatus,
    Function(FunctionCode),
}

impl AppTask {
    pub(crate) fn as_task_type(&self) -> TaskType {
        match self {
            AppTask::Read(t) => t.as_task_type(),
            AppTask::NonRead(t) => t.as_task_type(),
        }
    }

    pub(crate) fn function(&self) -> FunctionCode {
        match self {
            AppTask::Read(t) => t.function(),
            AppTask::NonRead(t) => t.function(),
        }
    }

    pub(crate) fn on_task_error(self, association: Option<&mut Association>, err: TaskError) {
        match self {
            Self::NonRead(task) => task.on_task_error(association, err),
            Self::Read(task) => task.on_task_error(association, err),
        }
    }

    pub(crate) fn get_id(&self) -> TaskId {
        match self {
            AppTask::Read(_) => TaskId::Function(FunctionCode::Read),
            AppTask::NonRead(t) => TaskId::Function(t.function()),
        }
    }
}

impl Task {
    pub(crate) fn on_task_error(self, association: Option<&mut Association>, err: TaskError) {
        match self {
            Self::App(task) => task.on_task_error(association, err),
            Self::LinkStatus(promise) => promise.complete(Err(err)),
        }
    }

    /// Perform operation before sending and check if the request should still be sent
    ///
    /// Returning Some means the task should proceed, returning None means
    /// the task was cancelled, forget about it.
    pub(crate) fn start(self, association: &mut Association) -> Option<Task> {
        if let Task::App(AppTask::NonRead(task)) = self {
            return task.start(association).map(|task| task.wrap());
        }

        Some(self)
    }

    pub(crate) fn get_id(&self) -> TaskId {
        match self {
            Task::LinkStatus(_) => TaskId::LinkStatus,
            Task::App(task) => task.get_id(),
        }
    }
}

pub(crate) trait RequestWriter {
    fn function(&self) -> FunctionCode;
    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError>;
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
    /// Read file from the outstation
    FileRead(FileReadTask),
    /// Send username/password and get back an auth key
    AuthFile(AuthFileTask),
    /// Open a file on the outstation
    OpenFile(OpenFileTask),
    /// Close a file on the outstation
    CloseFile(CloseFileTask),
    /// Write a file block
    WriteFileBlock(WriteBlockTask),
    /// get info about a file
    GetFileInfo(GetFileInfoTask),
}

impl RequestWriter for ReadTask {
    fn function(&self) -> FunctionCode {
        FunctionCode::Read
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        match self {
            ReadTask::PeriodicPoll(poll) => poll.format(writer)?,
            ReadTask::StartupIntegrity(classes) => classes.write(writer)?,
            ReadTask::EventScan(classes) => classes.write(writer)?,
            ReadTask::SingleRead(req) => req.format(writer)?,
        }
        Ok(())
    }
}

impl RequestWriter for NonReadTask {
    fn function(&self) -> FunctionCode {
        self.function()
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        match self {
            NonReadTask::Auto(t) => t.write(writer)?,
            NonReadTask::Command(t) => t.write(writer)?,
            NonReadTask::TimeSync(t) => t.write(writer)?,
            NonReadTask::Restart(_) => {}
            NonReadTask::DeadBands(t) => t.write(writer)?,
            NonReadTask::EmptyResponseTask(t) => t.write(writer)?,
            NonReadTask::FileRead(t) => t.write(writer)?,
            NonReadTask::GetFileInfo(t) => t.write(writer)?,
            NonReadTask::OpenFile(t) => t.write(writer)?,
            NonReadTask::CloseFile(t) => t.write(writer)?,
            NonReadTask::WriteFileBlock(t) => t.write(writer)?,
            NonReadTask::AuthFile(t) => t.write(writer)?,
        }
        Ok(())
    }
}

impl From<crate::app::format::WriteError> for TaskError {
    fn from(_: crate::app::format::WriteError) -> Self {
        TaskError::WriteError
    }
}

impl ReadTask {
    pub(crate) fn wrap(self) -> Task {
        Task::App(AppTask::Read(self))
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
        Task::App(AppTask::NonRead(self))
    }

    pub(crate) fn start(self, association: &mut Association) -> Option<NonReadTask> {
        match self {
            Self::Command(_) => Some(self),
            Self::Auto(_) => Some(self),
            Self::TimeSync(task) => task.start(association).map(|task| task.wrap()),
            Self::Restart(_) => Some(self),
            Self::DeadBands(_) => Some(self),
            Self::EmptyResponseTask(_) => Some(self),
            Self::FileRead(_) => Some(self),
            Self::GetFileInfo(_) => Some(self),
            Self::OpenFile(_) => Some(self),
            Self::CloseFile(_) => Some(self),
            Self::WriteFileBlock(_) => Some(self),
            Self::AuthFile(_) => Some(self),
        }
    }

    pub(crate) fn function(&self) -> FunctionCode {
        match self {
            Self::Command(task) => task.function(),
            Self::Auto(task) => task.function(),
            Self::TimeSync(task) => task.function(),
            Self::Restart(task) => task.function(),
            Self::DeadBands(task) => task.function(),
            Self::EmptyResponseTask(task) => task.function(),
            Self::FileRead(task) => task.function(),
            Self::GetFileInfo(task) => task.function(),
            Self::OpenFile(task) => task.function(),
            Self::CloseFile(task) => task.function(),
            Self::WriteFileBlock(task) => task.function(),
            Self::AuthFile(task) => task.function(),
        }
    }

    pub(crate) fn on_task_error(self, association: Option<&mut Association>, err: TaskError) {
        match self {
            Self::Command(task) => task.on_task_error(err),
            Self::TimeSync(task) => task.on_task_error(association, err),
            Self::Auto(task) => task.on_task_error(association, err),
            Self::Restart(task) => task.on_task_error(err),
            Self::DeadBands(task) => task.on_task_error(err),
            Self::EmptyResponseTask(task) => task.on_task_error(err),
            Self::FileRead(task) => task.on_task_error(err),
            Self::GetFileInfo(task) => task.on_task_error(err),
            Self::OpenFile(task) => task.on_task_error(err),
            Self::CloseFile(task) => task.on_task_error(err),
            Self::WriteFileBlock(task) => task.on_task_error(err),
            Self::AuthFile(task) => task.on_task_error(err),
        }
    }

    pub(crate) async fn handle_response(
        self,
        association: &mut Association,
        response: Response<'_>,
    ) -> Result<Option<NonReadTask>, TaskError> {
        match self {
            Self::Command(task) => task.handle(response),
            Self::Auto(task) => task.handle(association, response),
            Self::TimeSync(task) => task.handle(association, response),
            Self::Restart(task) => task.handle(response),
            Self::DeadBands(task) => task.handle(response),
            Self::EmptyResponseTask(task) => task.handle(response),
            Self::FileRead(task) => task.handle(response).await,
            Self::GetFileInfo(task) => task.handle(response),
            Self::OpenFile(task) => task.handle(response),
            Self::CloseFile(task) => task.handle(response),
            Self::WriteFileBlock(task) => task.handle(response),
            Self::AuthFile(task) => task.handle(response),
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
            Self::DeadBands(_) => TaskType::WriteDeadBands,
            Self::EmptyResponseTask(_) => TaskType::GenericEmptyResponse(self.function()),
            Self::FileRead(_) => TaskType::FileRead,
            Self::GetFileInfo(_) => TaskType::GetFileInfo,
            Self::AuthFile(_) => TaskType::GetFileInfo,
            Self::OpenFile(_) => TaskType::FileOpen,
            Self::CloseFile(_) => TaskType::FileClose,
            Self::WriteFileBlock(_) => TaskType::FileWriteBlock,
        }
    }
}
