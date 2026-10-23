// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::error::Error;

use crate::app::control::{CommandStatus, ControlCode};
use crate::app::{Iin, Iin2};
use crate::app::{ObjectParseError, ParseErrorCause, Shutdown};
use crate::link::error::LinkError;
use crate::link::EndpointAddress;
use crate::master::association::NoAssociation;
use crate::transport::TransportResponseError;

use tokio::sync::mpsc::error::SendError;
use tokio::sync::oneshot::error::RecvError;

use crate::app::attr::BadAttribute;
use crate::app::parse::parser::SingleHeaderError;
use crate::master::MasterChannelType;
use crate::util::session::{RunError, StopReason};

/// Errors that can occur when adding an association
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(feature = "ffi"), non_exhaustive)]
pub enum AssociationError {
    /// Master task has shutdown
    Shutdown,
    /// Specified address is already in use
    DuplicateAddress(EndpointAddress),
    /// Channel is not the correct type for that operation
    WrongChannelType {
        /// Actual type of the channel
        actual: MasterChannelType,
        /// Channel type required for the requested operation
        required: MasterChannelType,
    },
}

/// Errors that can occur while executing a master task
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BadEncoding {
    /// Attribute could not be encoded
    Attribute(BadAttribute),
    /// Control code whose fields encode to a byte that parses back to a different control code
    ControlCode(ControlCode),
}

impl std::fmt::Display for BadEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BadEncoding::Attribute(x) => write!(f, "Bad attribute encoding: {x}"),
            BadEncoding::ControlCode(x) => {
                write!(f, "Control code does not encode to the same value: {x}")
            }
        }
    }
}

/// Errors that can occur while executing a master task
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(feature = "ffi"), non_exhaustive)]
pub enum TaskError {
    /// There are too many user requests queued
    TooManyRequests,
    /// An error occurred at the link level
    Link(LinkError),
    /// An error occurred at the transport/app level
    Transport,
    /// Outstation returned IIN.2 bit(s) indicating failure
    RejectedByIin2(Iin),
    /// The objects in a response to the task's request could not be parsed, either because they
    /// are malformed or because they use an encoding the parser doesn't support
    MalformedResponse(ObjectParseError),
    /// The response contains unexpected or invalid headers or data that don't match the request
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
    /// Insufficient buffer space to serialize the request or data can't be encoded
    WriteError,
    /// Request data could not be encoded
    BadEncoding(BadEncoding),
    /// The requested association does not exist (not configured)
    NoSuchAssociation(EndpointAddress),
    /// There is not connection at the transport level
    NoConnection,
    /// The master was disabled or shutdown
    Shutdown,
    /// The master was disabled
    Disabled,
    /// The master had no system time to synchronize the outstation with, because the
    /// [`AssociationHandler`](crate::master::AssociationHandler) didn't provide one
    SystemTimeNotAvailable,
}

/// Errors that can occur when adding/modifying polls
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PollError {
    /// the master task has shutdown
    Shutdown,
    /// no association with the specified address exists
    NoSuchAssociation(EndpointAddress),
}

/// Errors that can occur when verifying the respond to a command request
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CommandResponseError {
    /// Command failed before receiving a response
    #[deprecated(since = "1.8.0", note = "never returned; will be removed in 2.0")]
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

/// Error type for operations that don't return anything from the outstation but might fail
/// b/c IIN2 has an error bit set
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WriteError {
    /// Error occurred during task execution
    Task(TaskError),
    /// Outstation returned an IIN.2 error
    #[deprecated(since = "1.8.0", note = "never returned; will be removed in 2.0")]
    IinError(Iin2),
}

/// Parent error type for time sync tasks
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TimeSyncError {
    /// Error occurred during task execution
    Task(TaskError),
    /// Clock rollback was detected
    ClockRollback,
    /// System time could not be converted to a UNIX timestamp
    #[deprecated(since = "1.8.0", note = "never returned; will be removed in 2.0")]
    SystemTimeNotUnix,
    /// Outstation time not return a value time delay
    BadOutstationTimeDelay(u16),
    /// Time calculation would overflow its representation
    Overflow,
    /// Outstation did not clear its NEED_TIME bit
    StillNeedsTime,
    /// System time was not available to perform the synchronization
    SystemTimeNotAvailable,
    /// Outstation returned an IIN.2 error
    #[deprecated(since = "1.8.0", note = "never returned; will be removed in 2.0")]
    IinError(Iin2),
}
/// Parent error type for command tasks
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
                "master already contains association with outstation address: {address}"
            ),
            AssociationError::WrongChannelType { actual, required } => write!(
                f,
                "operation requires master channel type to be {required:?} but it is {actual:?}"
            ),
        }
    }
}

impl std::fmt::Display for PollError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PollError::Shutdown => f.write_str("operation failed b/c the master has been shutdown"),
            PollError::NoSuchAssociation(address) => {
                write!(f, "no association with address: {address}")
            }
        }
    }
}

impl std::fmt::Display for CommandResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            #[expect(deprecated, reason = "Display must cover every variant until removal")]
            CommandResponseError::Request(err) => write!(f, "{err}"),
            CommandResponseError::BadStatus(status) => write!(
                f,
                "command status value other than Success was returned: {status:?}"
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
            TaskError::TooManyRequests => {
                f.write_str("the number of queued user requests has reached the configured limit")
            }
            TaskError::Link(_) => f.write_str("link-layer or I/O error"),
            TaskError::Transport => f.write_str("malformed response"),
            TaskError::MalformedResponse(err) => match err.cause() {
                ParseErrorCause::Unsupported => write!(f, "unsupported response content: {err}"),
                ParseErrorCause::Invalid => write!(f, "malformed response: {err}"),
                ParseErrorCause::UnsupportedOrInvalid => {
                    write!(f, "unsupported or invalid response content: {err}")
                }
            },
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
            TaskError::Disabled => f.write_str("the master was disabled while executing the task"),
            TaskError::NoConnection => f.write_str("no connection"),
            TaskError::NoSuchAssociation(x) => write!(f, "no association with address: {x}"),
            TaskError::BadEncoding(x) => {
                write!(f, "Encoding error: {x}")
            }
            TaskError::RejectedByIin2(iin) => {
                write!(f, "Rejected by IIN2: {}", iin.iin2)
            }
            TaskError::SystemTimeNotAvailable => f.write_str("system time not available"),
        }
    }
}

impl std::fmt::Display for TimeSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TimeSyncError::Task(err) => write!(f, "{err}"),
            #[expect(deprecated, reason = "Display must cover every variant until removal")]
            TimeSyncError::SystemTimeNotUnix => {
                f.write_str("the system time cannot be converted to unix time")
            }
            TimeSyncError::BadOutstationTimeDelay(x) => {
                write!(f, "outstation time delay ({x}) exceeded the response delay")
            }
            TimeSyncError::Overflow => f.write_str("overflow in calculation"),
            TimeSyncError::ClockRollback => f.write_str("detected a clock rollback"),
            TimeSyncError::StillNeedsTime => f.write_str("outstation did not clear NEED_TIME bit"),
            TimeSyncError::SystemTimeNotAvailable => f.write_str("system time not available"),
            #[expect(deprecated, reason = "Display must cover every variant until removal")]
            TimeSyncError::IinError(iin2) => write!(f, "outstation indicated an error: {iin2}"),
        }
    }
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WriteError::Task(err) => write!(f, "{err}"),
            #[expect(deprecated, reason = "Display must cover every variant until removal")]
            WriteError::IinError(iin2) => write!(f, "outstation indicated an error: {iin2}"),
        }
    }
}

impl From<scursor::WriteError> for TaskError {
    fn from(_: scursor::WriteError) -> Self {
        TaskError::WriteError
    }
}

impl From<LinkError> for TaskError {
    fn from(err: LinkError) -> Self {
        TaskError::Link(err)
    }
}

impl From<TransportResponseError> for TaskError {
    fn from(_: TransportResponseError) -> Self {
        TaskError::Transport
    }
}

impl From<ObjectParseError> for TaskError {
    fn from(err: ObjectParseError) -> Self {
        TaskError::MalformedResponse(err)
    }
}

impl From<SingleHeaderError> for TaskError {
    fn from(err: SingleHeaderError) -> Self {
        match err {
            SingleHeaderError::BadParse(err) => TaskError::MalformedResponse(err),
            SingleHeaderError::NoHeaders | SingleHeaderError::MoreThanOneHeader => {
                TaskError::UnexpectedResponseHeaders
            }
        }
    }
}

impl From<RunError> for TaskError {
    fn from(x: RunError) -> Self {
        match x {
            RunError::Stop(StopReason::Shutdown) => TaskError::Shutdown,
            RunError::Stop(StopReason::Disable) => TaskError::Disabled,
            RunError::Link(x) => TaskError::Link(x),
        }
    }
}

impl From<NoAssociation> for TaskError {
    fn from(x: NoAssociation) -> Self {
        TaskError::NoSuchAssociation(x.address)
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
        match err {
            TaskError::SystemTimeNotAvailable => TimeSyncError::SystemTimeNotAvailable,
            err => TimeSyncError::Task(err),
        }
    }
}

impl From<RecvError> for AssociationError {
    fn from(_: RecvError) -> Self {
        AssociationError::Shutdown
    }
}

impl From<RecvError> for WriteError {
    fn from(_: RecvError) -> Self {
        WriteError::Task(TaskError::Shutdown)
    }
}

impl From<TaskError> for WriteError {
    fn from(err: TaskError) -> Self {
        WriteError::Task(err)
    }
}

impl From<StopReason> for TaskError {
    fn from(x: StopReason) -> Self {
        match x {
            StopReason::Disable => TaskError::Disabled,
            StopReason::Shutdown => TaskError::Shutdown,
        }
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

impl From<Shutdown> for AssociationError {
    fn from(_: Shutdown) -> Self {
        AssociationError::Shutdown
    }
}

impl From<Shutdown> for TaskError {
    fn from(_: Shutdown) -> Self {
        TaskError::Shutdown
    }
}

impl From<Shutdown> for CommandError {
    fn from(_: Shutdown) -> Self {
        CommandError::Task(TaskError::Shutdown)
    }
}

impl From<Shutdown> for TimeSyncError {
    fn from(_: Shutdown) -> Self {
        TimeSyncError::Task(TaskError::Shutdown)
    }
}

impl From<Shutdown> for PollError {
    fn from(_: Shutdown) -> Self {
        PollError::Shutdown
    }
}

impl From<Shutdown> for WriteError {
    fn from(_: Shutdown) -> Self {
        WriteError::Task(TaskError::Shutdown)
    }
}

impl Error for AssociationError {}
impl Error for TaskError {}
impl Error for PollError {}
impl Error for CommandError {}
impl Error for CommandResponseError {}
impl Error for TimeSyncError {}
impl Error for WriteError {}

#[cfg(test)]
mod test {
    use super::*;
    use crate::app::variations::Variation;
    use crate::app::QualifierCode;

    #[test]
    fn time_sync_error_from_task_error_uses_the_dedicated_system_time_variant() {
        assert_eq!(
            TimeSyncError::from(TaskError::SystemTimeNotAvailable),
            TimeSyncError::SystemTimeNotAvailable
        );
        assert_eq!(
            TimeSyncError::from(TaskError::ResponseTimeout),
            TimeSyncError::Task(TaskError::ResponseTimeout)
        );
    }

    #[test]
    fn task_error_words_parse_failures_by_cause() {
        let unsupported =
            TaskError::MalformedResponse(ObjectParseError::UnknownGroupVariation(1, 9));
        assert_eq!(
            unsupported.to_string(),
            "unsupported response content: unknown group/variation: g1v9"
        );

        let either = TaskError::MalformedResponse(ObjectParseError::InvalidQualifierForVariation(
            Variation::Group111(5),
            QualifierCode::Range8,
        ));
        assert_eq!(
            either.to_string(),
            "unsupported or invalid response content: Group111(5) may not be used with the qualifier: 1-byte start/stop (0x00)"
        );

        let malformed = TaskError::MalformedResponse(ObjectParseError::InsufficientBytes);
        assert_eq!(
            malformed.to_string(),
            "malformed response: insufficient bytes for object header"
        );
    }
}
