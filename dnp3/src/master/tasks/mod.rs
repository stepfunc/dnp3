use std::future::Future;

use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::app::ResponseHeader;
use crate::master::association::Association;
use crate::master::error::TaskError;
use crate::master::extract::{extract_measurements, Delivered, ProcessableResponse};
use crate::master::poll::Poll;
use crate::master::promise::Promise;
use crate::master::request::{Classes, EventClasses};
use crate::master::tasks::auto::AutoTask;
use crate::master::tasks::command::CommandTask;
use crate::master::tasks::command_no_ack::CommandNoAckTask;
use crate::master::tasks::read::SingleReadTask;
use crate::master::tasks::restart::RestartType;
use crate::master::tasks::time::{PendingTimeSync, TimeSyncTask};
use crate::master::{CommandError, FileError, ReadType, TaskType, TimeSyncError, WriteError};

use crate::master::tasks::deadbands::DeadBandsRequest;
use crate::master::tasks::empty_response::EmptyResponseRequest;
use crate::master::tasks::file::authenticate::AuthFileRequest;
use crate::master::tasks::file::close::CloseFileRequest;
use crate::master::tasks::file::get_info::GetFileInfoRequest;
use crate::master::tasks::file::open::OpenFileRequest;
use crate::master::tasks::file::read::FileReadTask;
use crate::master::tasks::file::write_block::WriteBlockRequest;
use crate::transport::FragmentAddr;

pub(crate) mod auto;
pub(crate) mod command;
pub(crate) mod command_no_ack;
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
    /// Tasks whose request the outstation never responds to, complete once it is sent
    NoResponse(CommandNoAckTask),
}

pub(crate) enum Task {
    /// An application layer task
    App(AppTask),
    /// Send link status request
    LinkStatus(Promise<Result<(), TaskError>>),
}

/// A task waiting for the scheduler to start it
pub(crate) enum QueuedTask {
    /// Ready to send as is
    Ready(Task),
    /// Captures the time its first request is based on when it starts
    TimeSync(PendingTimeSync),
}

impl From<Task> for QueuedTask {
    fn from(task: Task) -> Self {
        Self::Ready(task)
    }
}

impl From<PendingTimeSync> for QueuedTask {
    fn from(task: PendingTimeSync) -> Self {
        Self::TimeSync(task)
    }
}

impl QueuedTask {
    pub(crate) fn on_task_error(self, association: Option<&mut Association>, err: TaskError) {
        match self {
            Self::Ready(task) => task.on_task_error(association, err),
            Self::TimeSync(task) => task.on_task_error(association, err),
        }
    }

    /// Perform any operation needed before sending and check that the task should still be sent
    ///
    /// Returning `None` means starting failed and the failure has already been reported.
    pub(crate) fn start(self, association: &mut Association) -> Option<Task> {
        match self {
            Self::Ready(task) => Some(task),
            Self::TimeSync(task) => task.start(association).map(Task::from),
        }
    }
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
            AppTask::NoResponse(_) => TaskType::Command,
        }
    }

    pub(crate) fn function(&self) -> FunctionCode {
        match self {
            AppTask::Read(t) => t.function(),
            AppTask::NonRead(t) => t.function(),
            AppTask::NoResponse(t) => t.function(),
        }
    }

    pub(crate) fn on_task_error(self, association: Option<&mut Association>, err: TaskError) {
        match self {
            Self::NonRead(task) => task.on_task_error(association, err),
            Self::Read(task) => task.on_task_error(association, err),
            Self::NoResponse(task) => task.on_task_error(err),
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

    pub(crate) fn get_id(&self) -> TaskId {
        match self {
            Task::LinkStatus(_) => TaskId::LinkStatus,
            Task::App(task) => TaskId::Function(task.function()),
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

/// How a failed read task is logged
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReadFailureLog {
    /// Log a warning that names the cause
    Warn(TaskError),
    /// Log nothing
    Nothing,
}

/// What a task does after it handles a response
pub(crate) enum Step<T> {
    /// send the next request
    Next,
    /// the task is complete
    Done(T),
}

/// A task that sends requests and handles their responses until it's done
///
/// The task never reports its own outcome: the session passes it to [`ResponseTask::complete`]
/// exactly once, whether the task succeeds, its response is bad, or it fails some other way.
pub(crate) trait ResponseTask: Send + Sized {
    /// What the task produces when it succeeds
    type Output: Send;
    /// Why the task failed, as its requester sees it
    type Error: RequestError;

    fn function(&self) -> FunctionCode;

    fn task_type(&self) -> TaskType;

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError>;

    /// Handle the response to the last request
    fn on_response(
        &mut self,
        association: &mut Association,
        response: Response<'_>,
    ) -> impl Future<Output = Result<Step<Self::Output>, Self::Error>> + Send;

    /// Report the outcome to whoever is waiting for it
    fn complete(
        self,
        association: Option<&mut Association>,
        result: Result<Self::Output, Self::Error>,
    );
}

/// Why a task failed, as its requester sees it
pub(crate) trait RequestError: Copy + Send + From<TaskError> {
    /// The error the session and the association see when the task fails
    fn task_error(self) -> TaskError;
}

impl RequestError for TaskError {
    fn task_error(self) -> TaskError {
        self
    }
}

impl RequestError for CommandError {
    fn task_error(self) -> TaskError {
        match self {
            CommandError::Task(err) => err,
            CommandError::Response(_) => TaskError::UnexpectedResponseHeaders,
        }
    }
}

impl RequestError for TimeSyncError {
    fn task_error(self) -> TaskError {
        match self {
            TimeSyncError::Task(err) => err,
            TimeSyncError::ClockRollback
            | TimeSyncError::BadOutstationTimeDelay(_)
            | TimeSyncError::Overflow
            | TimeSyncError::StillNeedsTime => TaskError::UnexpectedResponseHeaders,
            TimeSyncError::SystemTimeNotAvailable => TaskError::SystemTimeNotAvailable,
            #[expect(
                deprecated,
                reason = "the match must cover every variant until removal"
            )]
            TimeSyncError::SystemTimeNotUnix | TimeSyncError::IinError(_) => {
                TaskError::UnexpectedResponseHeaders
            }
        }
    }
}

impl RequestError for WriteError {
    fn task_error(self) -> TaskError {
        match self {
            WriteError::Task(err) => err,
            #[expect(
                deprecated,
                reason = "the match must cover every variant until removal"
            )]
            WriteError::IinError(_) => TaskError::UnexpectedResponseHeaders,
        }
    }
}

impl RequestError for FileError {
    fn task_error(self) -> TaskError {
        self.into()
    }
}

/// Handle a response, completing the task unless it has another request to send
async fn respond<T: ResponseTask>(
    mut task: T,
    association: &mut Association,
    response: Response<'_>,
) -> Result<Option<T>, TaskError> {
    match task.on_response(association, response).await {
        Ok(Step::Next) => Ok(Some(task)),
        Ok(Step::Done(output)) => {
            task.complete(Some(association), Ok(output));
            Ok(None)
        }
        Err(err) => {
            task.complete(Some(association), Err(err));
            Err(err.task_error())
        }
    }
}

fn fail<T: ResponseTask>(task: T, association: Option<&mut Association>, err: TaskError) {
    task.complete(association, Err(err.into()))
}

/// A request that one response answers
pub(crate) trait OneShotRequest: Send + Sized {
    type Output: Send;
    type Error: RequestError;

    fn function(&self) -> FunctionCode;

    fn task_type(&self) -> TaskType;

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError>;

    fn parse(&self, response: Response<'_>) -> Result<Self::Output, Self::Error>;

    fn wrap(task: OneShot<Self>) -> NonReadTask;
}

/// A task that sends one request and passes the parsed response to its requester
pub(crate) struct OneShot<R: OneShotRequest> {
    request: R,
    promise: Promise<Result<R::Output, R::Error>>,
}

impl<R: OneShotRequest> OneShot<R> {
    pub(crate) fn new(request: R, promise: Promise<Result<R::Output, R::Error>>) -> Self {
        Self { request, promise }
    }
}

impl<R: OneShotRequest> From<OneShot<R>> for Task {
    fn from(value: OneShot<R>) -> Self {
        Task::App(AppTask::NonRead(R::wrap(value)))
    }
}

impl<R: OneShotRequest> ResponseTask for OneShot<R> {
    type Output = R::Output;
    type Error = R::Error;

    fn function(&self) -> FunctionCode {
        self.request.function()
    }

    fn task_type(&self) -> TaskType {
        self.request.task_type()
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        self.request.write(writer)
    }

    async fn on_response(
        &mut self,
        _association: &mut Association,
        response: Response<'_>,
    ) -> Result<Step<R::Output>, R::Error> {
        self.request.parse(response).map(Step::Done)
    }

    fn complete(self, _association: Option<&mut Association>, result: Result<R::Output, R::Error>) {
        self.promise.complete(result)
    }
}

pub(crate) enum NonReadTask {
    /// tasks that occur automatically during startup, or based on events or configuration,
    Auto(AutoTask),
    /// commands initiated from the user API
    Command(CommandTask),
    /// time synchronization
    TimeSync(TimeSyncTask),
    /// restart operation
    Restart(OneShot<RestartType>),
    /// write dead-bands
    DeadBands(OneShot<DeadBandsRequest>),
    /// Generic task for anything that doesn't have response object headers
    EmptyResponseTask(OneShot<EmptyResponseRequest>),
    /// Read file from the outstation
    FileRead(FileReadTask),
    /// Send username/password and get back an auth key
    AuthFile(OneShot<AuthFileRequest>),
    /// Open a file on the outstation
    OpenFile(OneShot<OpenFileRequest>),
    /// Close a file on the outstation
    CloseFile(OneShot<CloseFileRequest>),
    /// Write a file block
    WriteFileBlock(OneShot<WriteBlockRequest>),
    /// get info about a file
    GetFileInfo(OneShot<GetFileInfoRequest>),
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
            NonReadTask::Auto(t) => t.write(writer),
            NonReadTask::Command(t) => t.write(writer),
            NonReadTask::TimeSync(t) => t.write(writer),
            NonReadTask::Restart(t) => t.write(writer),
            NonReadTask::DeadBands(t) => t.write(writer),
            NonReadTask::EmptyResponseTask(t) => t.write(writer),
            NonReadTask::FileRead(t) => t.write(writer),
            NonReadTask::GetFileInfo(t) => t.write(writer),
            NonReadTask::OpenFile(t) => t.write(writer),
            NonReadTask::CloseFile(t) => t.write(writer),
            NonReadTask::WriteFileBlock(t) => t.write(writer),
            NonReadTask::AuthFile(t) => t.write(writer),
        }
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

    /// Pass the response's measurements to the handler, returning what was delivered
    pub(crate) async fn process_response(
        &mut self,
        association: &mut Association,
        header: ResponseHeader,
        objects: ProcessableResponse<'_>,
    ) -> Delivered {
        let read_type = self.read_type();
        match self {
            ReadTask::SingleRead(SingleReadTask {
                custom_handler: Some(handler),
                ..
            }) => extract_measurements(read_type, header, objects, handler.as_mut()).await,
            _ => {
                association
                    .handle_read_response(read_type, header, objects)
                    .await
            }
        }
    }

    /// The ReadType reported to the ReadHandler for this read's responses
    fn read_type(&self) -> ReadType {
        match self {
            ReadTask::StartupIntegrity(_) => ReadType::StartupIntegrity,
            ReadTask::PeriodicPoll(_) | ReadTask::EventScan(_) => ReadType::PeriodicPoll,
            ReadTask::SingleRead(_) => ReadType::SinglePoll,
        }
    }

    /// True if the read fails when the outstation reports an error and delivers no data. The
    /// automatic tasks complete regardless, because retrying can't change the outstation's answer,
    /// and their retry backoff would hold up polling
    pub(crate) fn fails_without_data(&self) -> bool {
        match self {
            Self::SingleRead(_) | Self::PeriodicPoll(_) => true,
            Self::StartupIntegrity(_) | Self::EventScan(_) => false,
        }
    }

    /// Complete the read, given whether it returned events without an IIN2 request error
    pub(crate) fn complete(
        self,
        association: &mut Association,
        events_without_request_errors: bool,
    ) {
        match self {
            ReadTask::StartupIntegrity(_) => association.on_integrity_scan_complete(),
            ReadTask::PeriodicPoll(poll) => association.complete_poll(&poll),
            ReadTask::EventScan(_) => {
                association.on_event_scan_complete(events_without_request_errors)
            }
            ReadTask::SingleRead(task) => task.on_complete(),
        }
    }

    /// How a failure of this task is logged. User reads don't log a shutdown or disable, which
    /// fails every queued read
    fn failure_log(&self, err: TaskError) -> ReadFailureLog {
        match self {
            ReadTask::StartupIntegrity(_) | ReadTask::PeriodicPoll(_) | ReadTask::EventScan(_) => {
                ReadFailureLog::Warn(err)
            }
            ReadTask::SingleRead(_) => match err {
                TaskError::Shutdown | TaskError::Disabled => ReadFailureLog::Nothing,
                TaskError::TooManyRequests
                | TaskError::Link(_)
                | TaskError::Transport
                | TaskError::RejectedByIin2(_)
                | TaskError::MalformedResponse(_)
                | TaskError::UnexpectedResponseHeaders
                | TaskError::NonFinWithoutCon
                | TaskError::NeverReceivedFir
                | TaskError::UnexpectedFir
                | TaskError::MultiFragmentResponse
                | TaskError::ResponseTimeout
                | TaskError::WriteError
                | TaskError::BadEncoding(_)
                | TaskError::NoSuchAssociation(_)
                | TaskError::NoConnection
                | TaskError::SystemTimeNotAvailable => ReadFailureLog::Warn(err),
            },
        }
    }

    fn log_failure(&self, err: TaskError) {
        let ReadFailureLog::Warn(cause) = self.failure_log(err) else {
            return;
        };
        match self {
            ReadTask::StartupIntegrity(_) => {
                tracing::warn!("startup integrity scan failed: {cause}")
            }
            ReadTask::PeriodicPoll(poll) => tracing::warn!("poll {} failed: {cause}", poll.id),
            ReadTask::EventScan(_) => tracing::warn!("automatic event scan failed: {cause}"),
            ReadTask::SingleRead(_) => tracing::warn!("read failed: {cause}"),
        }
    }

    pub(crate) fn on_task_error(self, association: Option<&mut Association>, err: TaskError) {
        self.log_failure(err);
        match self {
            ReadTask::StartupIntegrity(_) => {
                if let Some(association) = association {
                    association.on_integrity_scan_failure();
                }
            }
            ReadTask::PeriodicPoll(poll) => {
                if let Some(association) = association {
                    association.complete_poll(&poll);
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
            Self::Command(task) => fail(task, association, err),
            Self::TimeSync(task) => fail(task, association, err),
            Self::Auto(task) => fail(task, association, err),
            Self::Restart(task) => fail(task, association, err),
            Self::DeadBands(task) => fail(task, association, err),
            Self::EmptyResponseTask(task) => fail(task, association, err),
            Self::FileRead(task) => fail(task, association, err),
            Self::GetFileInfo(task) => fail(task, association, err),
            Self::OpenFile(task) => fail(task, association, err),
            Self::CloseFile(task) => fail(task, association, err),
            Self::WriteFileBlock(task) => fail(task, association, err),
            Self::AuthFile(task) => fail(task, association, err),
        }
    }

    pub(crate) async fn handle_response(
        self,
        association: &mut Association,
        response: Response<'_>,
    ) -> Result<Option<NonReadTask>, TaskError> {
        let next = match self {
            Self::Command(task) => respond(task, association, response)
                .await?
                .map(Self::Command),
            Self::Auto(task) => respond(task, association, response).await?.map(Self::Auto),
            Self::TimeSync(task) => respond(task, association, response)
                .await?
                .map(Self::TimeSync),
            Self::Restart(task) => respond(task, association, response)
                .await?
                .map(Self::Restart),
            Self::DeadBands(task) => respond(task, association, response)
                .await?
                .map(Self::DeadBands),
            Self::EmptyResponseTask(task) => respond(task, association, response)
                .await?
                .map(Self::EmptyResponseTask),
            Self::FileRead(task) => respond(task, association, response)
                .await?
                .map(Self::FileRead),
            Self::GetFileInfo(task) => respond(task, association, response)
                .await?
                .map(Self::GetFileInfo),
            Self::OpenFile(task) => respond(task, association, response)
                .await?
                .map(Self::OpenFile),
            Self::CloseFile(task) => respond(task, association, response)
                .await?
                .map(Self::CloseFile),
            Self::WriteFileBlock(task) => respond(task, association, response)
                .await?
                .map(Self::WriteFileBlock),
            Self::AuthFile(task) => respond(task, association, response)
                .await?
                .map(Self::AuthFile),
        };
        Ok(next)
    }

    pub(crate) fn as_task_type(&self) -> TaskType {
        match self {
            Self::Command(task) => task.task_type(),
            Self::Auto(task) => task.task_type(),
            Self::TimeSync(task) => task.task_type(),
            Self::Restart(task) => task.task_type(),
            Self::DeadBands(task) => task.task_type(),
            Self::EmptyResponseTask(task) => task.task_type(),
            Self::FileRead(task) => task.task_type(),
            Self::GetFileInfo(task) => task.task_type(),
            Self::AuthFile(task) => task.task_type(),
            Self::OpenFile(task) => task.task_type(),
            Self::CloseFile(task) => task.task_type(),
            Self::WriteFileBlock(task) => task.task_type(),
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use super::*;
    use crate::app::ObjectParseError;
    use crate::master::request::ReadRequest;

    fn read_tasks() -> [ReadTask; 4] {
        [
            ReadTask::StartupIntegrity(Classes::all()),
            ReadTask::PeriodicPoll(Poll::new(
                0,
                ReadRequest::class_scan(Classes::class123()),
                Duration::from_secs(1),
            )),
            ReadTask::EventScan(EventClasses::all()),
            ReadTask::SingleRead(SingleReadTask::new(
                ReadRequest::class_scan(Classes::all()),
                Promise::null(),
            )),
        ]
    }

    #[test]
    fn every_read_task_warns_with_the_cause() {
        let causes = [
            TaskError::MalformedResponse(ObjectParseError::InsufficientBytes),
            TaskError::ResponseTimeout,
            TaskError::TooManyRequests,
            TaskError::NoConnection,
        ];
        for task in read_tasks() {
            for cause in causes {
                assert_eq!(
                    task.failure_log(cause),
                    ReadFailureLog::Warn(cause),
                    "{:?}",
                    task.as_task_type()
                );
            }
        }
    }

    #[test]
    fn only_user_reads_skip_shutdown_and_disable() {
        for task in read_tasks() {
            let user_read = matches!(task, ReadTask::SingleRead(_));
            for cause in [TaskError::Shutdown, TaskError::Disabled] {
                let expected = if user_read {
                    ReadFailureLog::Nothing
                } else {
                    ReadFailureLog::Warn(cause)
                };
                assert_eq!(
                    task.failure_log(cause),
                    expected,
                    "{:?}",
                    task.as_task_type()
                );
            }
        }
    }
}
