use crate::app::enums::CommandStatus;
use crate::app::header::{Iin, Iin2};
use crate::app::parse::error::ObjectParseError;
use crate::link::error::LinkError;
use crate::link::EndpointAddress;
use crate::master::association::NoAssociation;
use crate::master::session::RunError;
use crate::tokio::sync::mpsc::error::SendError;
use crate::tokio::sync::oneshot::error::RecvError;
use crate::util::cursor::WriteError;
use crate::util::task::Shutdown;
use std::error::Error;

/// Errors that can occur when adding an association
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AssociationError {
    /// the master task has shutdown
    Shutdown,
    /// the specified address is already in use
    DuplicateAddress(EndpointAddress),
}

/// Errors that can occur while executing a master task
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TaskError {
    /// An error occurred at the link or transport level
    Lower(LinkError),
    /// A response to the task's request was malformed
    MalformedResponse(ObjectParseError),
    /// The response contains headers that don't match the request
    UnexpectedResponseHeaders,
    /// Non-final response not requesting confirmation
    NonFinWithoutCon,
    /// Received a non-FIR response when expecting the FIR bit
    NeverReceivedFir,
    /// Received FIR bit after already receiving FIR
    UnexpectedFir,
    /// Received a multi-fragmented response when expecting FIR/FIN
    MultiFragmentResponse,
    /// The response timed-out
    ResponseTimeout,
    /// Insufficient buffer space to serialize the request
    WriteError,
    /// The requested association does not exist (not configured)
    NoSuchAssociation(EndpointAddress),
    /// There is not connection at the transport level
    NoConnection,
    /// The master was shutdown
    Shutdown,
}

// Errors that can occur when adding/modifying polls
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PollError {
    /// the master task has shutdown
    Shutdown,
    /// no association with the specified address exists
    NoSuchAssociation(EndpointAddress),
}

/// Errors that can occur when verifying the respond to a command request
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CommandResponseError {
    /// Command failed before receiving a response
    Request(TaskError),
    /// Outstation indicated that a command was not SUCCESS for the specified reason
    BadStatus(CommandStatus),
    /// Number of headers in the response doesn't match the number in the request
    HeaderCountMismatch,
    /// Header in the response doesn't match the request
    HeaderTypeMismatch,
    /// Number of objects in one of the headers doesn't match the request
    ObjectCountMismatch,
    /// Value in one of the objects in the response doesn't match the request
    ObjectValueMismatch,
}

/// Parent error type for time sync tasks
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TimeSyncError {
    Task(TaskError),
    ClockRollback,
    SystemTimeNotUnix,
    BadOutstationTimeDelay(u16),
    Overflow,
    StillNeedsTime,
    SystemTimeNotAvailable,
    IinError(Iin2),
}

impl TimeSyncError {
    pub(crate) fn from_iin(iin: Iin) -> Result<(), TimeSyncError> {
        if iin.iin1.get_need_time() {
            return Err(TimeSyncError::StillNeedsTime);
        }
        if iin.has_request_error() {
            return Err(TimeSyncError::IinError(iin.iin2));
        }
        Ok(())
    }
}

/// Parent error type for command tasks
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CommandError {
    /// Failed b/c of a generic task execution error
    Task(TaskError),
    /// Failed b/c of an unexpected response to Select, Operate, or DirectOperate
    Response(CommandResponseError),
}

impl std::fmt::Display for AssociationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AssociationError::Shutdown => {
                f.write_str("operation failed b/c the master has been shutdown")
            }
            AssociationError::DuplicateAddress(address) => write!(
                f,
                "master already contains association with outstation address: {}",
                address
            ),
        }
    }
}

impl std::fmt::Display for PollError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PollError::Shutdown => f.write_str("operation failed b/c the master has been shutdown"),
            PollError::NoSuchAssociation(address) => {
                write!(f, "no association with address: {}", address)
            }
        }
    }
}

impl std::fmt::Display for CommandResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CommandResponseError::Request(err) => write!(f, "{}", err),
            CommandResponseError::BadStatus(status) => write!(
                f,
                "command status value other than Success was returned: {:?}",
                status
            ),
            CommandResponseError::HeaderCountMismatch => f.write_str(
                "response did not contain the same number of object headers as the request",
            ),
            CommandResponseError::HeaderTypeMismatch => {
                f.write_str("response contained a header type different than the request")
            }
            CommandResponseError::ObjectCountMismatch => f.write_str(
                "response header does not have the same number of objects as the request",
            ),
            CommandResponseError::ObjectValueMismatch => f.write_str(
                "a value other than the status is different in the response than the request",
            ),
        }
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CommandError::Response(x) => std::fmt::Display::fmt(x, f),
            CommandError::Task(x) => std::fmt::Display::fmt(x, f),
        }
    }
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TaskError::Lower(_) => f.write_str("I/O error"),
            TaskError::MalformedResponse(err) => write!(f, "malformed response: {}", err),
            TaskError::UnexpectedResponseHeaders => {
                f.write_str("response contains headers that don't match the request")
            }
            TaskError::NonFinWithoutCon => {
                f.write_str("outstation responses with FIN == 0 must request confirmation")
            }
            TaskError::NeverReceivedFir => {
                f.write_str("received non-FIR response before receiving FIR")
            }
            TaskError::UnexpectedFir => {
                f.write_str("received FIR bit after already receiving FIR bit")
            }
            TaskError::MultiFragmentResponse => {
                f.write_str("received unexpected multi-fragment response")
            }
            TaskError::ResponseTimeout => f.write_str("no response received within timeout"),
            TaskError::WriteError => {
                f.write_str("unable to serialize the task's request (insufficient buffer space)")
            }
            TaskError::Shutdown => f.write_str("the master was shutdown while executing the task"),
            TaskError::NoConnection => f.write_str("no connection"),
            TaskError::NoSuchAssociation(x) => write!(f, "no association with address: {}", x),
        }
    }
}

impl std::fmt::Display for TimeSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TimeSyncError::Task(err) => write!(f, "{}", err),
            TimeSyncError::SystemTimeNotUnix => {
                f.write_str("the system time cannot be converted to unix time")
            }
            TimeSyncError::BadOutstationTimeDelay(x) => write!(
                f,
                "outstation time delay ({}) exceeded the response delay",
                x
            ),
            TimeSyncError::Overflow => f.write_str("overflow in calculation"),
            TimeSyncError::ClockRollback => f.write_str("detected a clock rollback"),
            TimeSyncError::StillNeedsTime => f.write_str("outstation did not clear NEED_TIME bit"),
            TimeSyncError::SystemTimeNotAvailable => f.write_str("system time not available"),
            TimeSyncError::IinError(iin2) => write!(f, "outstation indicated an error: {}", iin2),
        }
    }
}

impl From<WriteError> for TaskError {
    fn from(_: WriteError) -> Self {
        TaskError::WriteError
    }
}

impl From<LinkError> for TaskError {
    fn from(err: LinkError) -> Self {
        TaskError::Lower(err)
    }
}

impl From<ObjectParseError> for TaskError {
    fn from(err: ObjectParseError) -> Self {
        TaskError::MalformedResponse(err)
    }
}

impl From<Shutdown> for TaskError {
    fn from(_: Shutdown) -> Self {
        TaskError::Shutdown
    }
}

impl From<RunError> for TaskError {
    fn from(x: RunError) -> Self {
        match x {
            RunError::Shutdown => TaskError::Shutdown,
            RunError::Link(x) => TaskError::Lower(x),
        }
    }
}

impl From<NoAssociation> for TaskError {
    fn from(x: NoAssociation) -> Self {
        TaskError::NoSuchAssociation(x.address)
    }
}

impl From<Shutdown> for PollError {
    fn from(_: Shutdown) -> Self {
        PollError::Shutdown
    }
}

impl From<NoAssociation> for PollError {
    fn from(x: NoAssociation) -> Self {
        PollError::NoSuchAssociation(x.address)
    }
}

impl From<RecvError> for PollError {
    fn from(_: RecvError) -> Self {
        PollError::Shutdown
    }
}

impl From<CommandResponseError> for CommandError {
    fn from(err: CommandResponseError) -> Self {
        CommandError::Response(err)
    }
}

impl From<TaskError> for CommandError {
    fn from(err: TaskError) -> Self {
        CommandError::Task(err)
    }
}

impl From<TaskError> for TimeSyncError {
    fn from(err: TaskError) -> Self {
        TimeSyncError::Task(err)
    }
}

impl From<RecvError> for AssociationError {
    fn from(_: RecvError) -> Self {
        AssociationError::Shutdown
    }
}

impl From<RecvError> for TaskError {
    fn from(_: RecvError) -> Self {
        TaskError::Shutdown
    }
}

impl From<RecvError> for CommandError {
    fn from(_: RecvError) -> Self {
        CommandError::Task(TaskError::Shutdown)
    }
}

impl From<RecvError> for TimeSyncError {
    fn from(_: RecvError) -> Self {
        TimeSyncError::Task(TaskError::Shutdown)
    }
}

impl<T> From<SendError<T>> for Shutdown {
    fn from(_: SendError<T>) -> Self {
        Shutdown
    }
}

impl<T> From<SendError<T>> for AssociationError {
    fn from(_: SendError<T>) -> Self {
        AssociationError::Shutdown
    }
}

impl<T> From<SendError<T>> for TaskError {
    fn from(_: SendError<T>) -> Self {
        TaskError::Shutdown
    }
}

impl<T> From<SendError<T>> for CommandError {
    fn from(_: SendError<T>) -> Self {
        CommandError::Task(TaskError::Shutdown)
    }
}

impl<T> From<SendError<T>> for TimeSyncError {
    fn from(_: SendError<T>) -> Self {
        TimeSyncError::Task(TaskError::Shutdown)
    }
}

impl<T> From<SendError<T>> for PollError {
    fn from(_: SendError<T>) -> Self {
        PollError::Shutdown
    }
}

impl Error for AssociationError {}
impl Error for TaskError {}
impl Error for PollError {}
impl Error for CommandError {}
impl Error for CommandResponseError {}
impl Error for TimeSyncError {}
