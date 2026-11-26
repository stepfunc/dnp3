// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::error::Error;

use crate::app::attr::{AttrWriteError, BadAttribute};
use crate::app::control::{CommandStatus, ControlCode};
use crate::app::parse::parser::SingleHeaderError;
use crate::app::{Iin, ObjectParseError, ParseErrorCause, Shutdown, Variation};
use crate::link::error::LinkError;
use crate::link::EndpointAddress;
use crate::master::association::NoAssociation;
use crate::master::MasterChannelType;
use crate::transport::TransportResponseError;
use crate::util::lifecycle::{RunError, StopReason};

/// Errors that can occur when adding an association
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AssociationError(pub(crate) AssociationErrorKind);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum AssociationErrorKind {
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

/// Errors that can occur when encoding a master request
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum BadEncoding {
    /// Attribute could not be encoded
    Attribute(BadAttribute),
    /// Control code whose fields encode to a byte that parses back to a different control code
    ControlCode(ControlCode),
    /// Variation that names no object, such as group 0 variation 0, which a READ can't request
    UndefinedVariation(Variation),
}

impl std::fmt::Display for BadEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BadEncoding::Attribute(x) => write!(f, "Bad attribute encoding: {x}"),
            BadEncoding::ControlCode(x) => {
                write!(f, "Control code does not encode to the same value: {x}")
            }
            BadEncoding::UndefinedVariation(x) => {
                write!(f, "{x} is not a defined object variation")
            }
        }
    }
}

/// Errors that can occur while executing a master task
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TaskError(pub(crate) TaskErrorKind);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum TaskErrorKind {
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
pub struct PollError(pub(crate) PollErrorKind);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum PollErrorKind {
    /// the master task has shutdown
    Shutdown,
    /// no association with the specified address exists
    NoSuchAssociation(EndpointAddress),
}

/// Errors that can occur when verifying the response to a command request
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum CommandResponseError {
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
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TimeSyncError(pub(crate) TimeSyncErrorKind);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum TimeSyncErrorKind {
    /// Error occurred during task execution
    Task(TaskErrorKind),
    /// Clock rollback was detected
    ClockRollback,
    /// Outstation time not return a value time delay
    BadOutstationTimeDelay(u16),
    /// Time calculation would overflow its representation
    Overflow,
    /// Outstation did not clear its NEED_TIME bit
    StillNeedsTime,
}

/// Parent error type for command tasks
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CommandError(pub(crate) CommandErrorKind);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum CommandErrorKind {
    /// Failed b/c of a generic task execution error
    Task(TaskErrorKind),
    /// Failed b/c of an unexpected response to Select, Operate, or DirectOperate
    Response(CommandResponseError),
}

impl CommandError {
    /// Returns the command status if the outstation rejected a command with a non-success status
    pub fn command_status(&self) -> Option<CommandStatus> {
        match self.0 {
            CommandErrorKind::Response(CommandResponseError::BadStatus(status)) => Some(status),
            CommandErrorKind::Task(_) | CommandErrorKind::Response(_) => None,
        }
    }
}

impl std::fmt::Display for AssociationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            AssociationErrorKind::Shutdown => {
                f.write_str("operation failed b/c the master has been shutdown")
            }
            AssociationErrorKind::DuplicateAddress(address) => write!(
                f,
                "master already contains association with outstation address: {address}"
            ),
            AssociationErrorKind::WrongChannelType { actual, required } => write!(
                f,
                "operation requires master channel type to be {required:?} but it is {actual:?}"
            ),
        }
    }
}

impl std::fmt::Display for PollError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            PollErrorKind::Shutdown => {
                f.write_str("operation failed b/c the master has been shutdown")
            }
            PollErrorKind::NoSuchAssociation(address) => {
                write!(f, "no association with address: {address}")
            }
        }
    }
}

impl std::fmt::Display for CommandResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            CommandErrorKind::Response(x) => std::fmt::Display::fmt(&x, f),
            CommandErrorKind::Task(x) => std::fmt::Display::fmt(&x, f),
        }
    }
}

impl std::fmt::Display for TaskErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskErrorKind::TooManyRequests => {
                f.write_str("the number of queued user requests has reached the configured limit")
            }
            TaskErrorKind::Link(err) => write!(f, "link-layer or I/O error: {err}"),
            TaskErrorKind::Transport => f.write_str("malformed response"),
            TaskErrorKind::MalformedResponse(err) => match err.cause() {
                ParseErrorCause::Unsupported => write!(f, "unsupported response content: {err}"),
                ParseErrorCause::Invalid => write!(f, "malformed response: {err}"),
                ParseErrorCause::UnsupportedOrInvalid => {
                    write!(f, "unsupported or invalid response content: {err}")
                }
            },
            TaskErrorKind::UnexpectedResponseHeaders => {
                f.write_str("response contains headers that don't match the request")
            }
            TaskErrorKind::NonFinWithoutCon => {
                f.write_str("outstation responses with FIN == 0 must request confirmation")
            }
            TaskErrorKind::NeverReceivedFir => {
                f.write_str("received non-FIR response before receiving FIR")
            }
            TaskErrorKind::UnexpectedFir => {
                f.write_str("received FIR bit after already receiving FIR bit")
            }
            TaskErrorKind::MultiFragmentResponse => {
                f.write_str("received unexpected multi-fragment response")
            }
            TaskErrorKind::ResponseTimeout => f.write_str("no response received within timeout"),
            TaskErrorKind::WriteError => {
                f.write_str("unable to serialize the task's request (insufficient buffer space)")
            }
            TaskErrorKind::Shutdown => {
                f.write_str("the master was shutdown while executing the task")
            }
            TaskErrorKind::Disabled => {
                f.write_str("the master was disabled while executing the task")
            }
            TaskErrorKind::NoConnection => f.write_str("no connection"),
            TaskErrorKind::NoSuchAssociation(x) => write!(f, "no association with address: {x}"),
            TaskErrorKind::BadEncoding(x) => {
                write!(f, "Encoding error: {x}")
            }
            TaskErrorKind::RejectedByIin2(iin) => {
                write!(f, "Rejected by IIN2: {}", iin.iin2)
            }
            TaskErrorKind::SystemTimeNotAvailable => f.write_str("system time not available"),
        }
    }
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::fmt::Display for TimeSyncErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeSyncErrorKind::Task(err) => std::fmt::Display::fmt(err, f),
            TimeSyncErrorKind::BadOutstationTimeDelay(x) => {
                write!(f, "outstation time delay ({x}) exceeded the response delay")
            }
            TimeSyncErrorKind::Overflow => f.write_str("overflow in calculation"),
            TimeSyncErrorKind::ClockRollback => f.write_str("detected a clock rollback"),
            TimeSyncErrorKind::StillNeedsTime => {
                f.write_str("outstation did not clear NEED_TIME bit")
            }
        }
    }
}

impl std::fmt::Display for TimeSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl From<AssociationErrorKind> for AssociationError {
    fn from(kind: AssociationErrorKind) -> Self {
        Self(kind)
    }
}

impl From<TaskErrorKind> for TaskError {
    fn from(kind: TaskErrorKind) -> Self {
        Self(kind)
    }
}

impl From<ObjectParseError> for TaskError {
    fn from(err: ObjectParseError) -> Self {
        Self(err.into())
    }
}

impl From<BadEncoding> for TaskError {
    fn from(err: BadEncoding) -> Self {
        Self(err.into())
    }
}

impl From<PollErrorKind> for PollError {
    fn from(kind: PollErrorKind) -> Self {
        Self(kind)
    }
}

impl From<CommandErrorKind> for CommandError {
    fn from(kind: CommandErrorKind) -> Self {
        Self(kind)
    }
}

impl From<CommandResponseError> for CommandError {
    fn from(err: CommandResponseError) -> Self {
        Self(err.into())
    }
}

impl From<TaskErrorKind> for CommandError {
    fn from(err: TaskErrorKind) -> Self {
        Self(err.into())
    }
}

impl From<TimeSyncErrorKind> for TimeSyncError {
    fn from(kind: TimeSyncErrorKind) -> Self {
        Self(kind)
    }
}

impl From<TaskErrorKind> for TimeSyncError {
    fn from(err: TaskErrorKind) -> Self {
        Self(err.into())
    }
}

impl From<BadEncoding> for TaskErrorKind {
    fn from(err: BadEncoding) -> Self {
        TaskErrorKind::BadEncoding(err)
    }
}

impl From<AttrWriteError> for TaskErrorKind {
    fn from(value: AttrWriteError) -> Self {
        match value {
            AttrWriteError::Cursor => TaskErrorKind::WriteError,
            AttrWriteError::BadAttribute(x) => BadEncoding::Attribute(x).into(),
        }
    }
}

impl From<scursor::WriteError> for TaskErrorKind {
    fn from(_: scursor::WriteError) -> Self {
        TaskErrorKind::WriteError
    }
}

impl From<LinkError> for TaskErrorKind {
    fn from(err: LinkError) -> Self {
        TaskErrorKind::Link(err)
    }
}

impl From<TransportResponseError> for TaskErrorKind {
    fn from(_: TransportResponseError) -> Self {
        TaskErrorKind::Transport
    }
}

impl From<ObjectParseError> for TaskErrorKind {
    fn from(err: ObjectParseError) -> Self {
        TaskErrorKind::MalformedResponse(err)
    }
}

impl From<SingleHeaderError> for TaskErrorKind {
    fn from(err: SingleHeaderError) -> Self {
        match err {
            SingleHeaderError::BadParse(err) => TaskErrorKind::MalformedResponse(err),
            SingleHeaderError::NoHeaders | SingleHeaderError::MoreThanOneHeader => {
                TaskErrorKind::UnexpectedResponseHeaders
            }
        }
    }
}

impl From<RunError> for TaskErrorKind {
    fn from(x: RunError) -> Self {
        match x {
            RunError::Stop(StopReason::Shutdown) => TaskErrorKind::Shutdown,
            RunError::Stop(StopReason::Disable) => TaskErrorKind::Disabled,
            RunError::Link(x) => TaskErrorKind::Link(x),
        }
    }
}

impl From<NoAssociation> for TaskErrorKind {
    fn from(x: NoAssociation) -> Self {
        TaskErrorKind::NoSuchAssociation(x.address)
    }
}

impl From<NoAssociation> for PollErrorKind {
    fn from(x: NoAssociation) -> Self {
        PollErrorKind::NoSuchAssociation(x.address)
    }
}

impl From<CommandResponseError> for CommandErrorKind {
    fn from(err: CommandResponseError) -> Self {
        CommandErrorKind::Response(err)
    }
}

impl From<TaskErrorKind> for CommandErrorKind {
    fn from(err: TaskErrorKind) -> Self {
        CommandErrorKind::Task(err)
    }
}

impl From<TaskError> for CommandError {
    fn from(err: TaskError) -> Self {
        CommandError(CommandErrorKind::Task(err.0))
    }
}

impl From<TaskErrorKind> for TimeSyncErrorKind {
    fn from(err: TaskErrorKind) -> Self {
        TimeSyncErrorKind::Task(err)
    }
}

impl From<TaskError> for TimeSyncError {
    fn from(err: TaskError) -> Self {
        TimeSyncError(TimeSyncErrorKind::Task(err.0))
    }
}

impl From<StopReason> for TaskErrorKind {
    fn from(x: StopReason) -> Self {
        match x {
            StopReason::Disable => TaskErrorKind::Disabled,
            StopReason::Shutdown => TaskErrorKind::Shutdown,
        }
    }
}

impl From<Shutdown> for AssociationError {
    fn from(_: Shutdown) -> Self {
        AssociationError(AssociationErrorKind::Shutdown)
    }
}

impl From<Shutdown> for TaskError {
    fn from(_: Shutdown) -> Self {
        TaskError(TaskErrorKind::Shutdown)
    }
}

impl From<Shutdown> for CommandError {
    fn from(_: Shutdown) -> Self {
        TaskErrorKind::Shutdown.into()
    }
}

impl From<Shutdown> for TimeSyncError {
    fn from(_: Shutdown) -> Self {
        TaskErrorKind::Shutdown.into()
    }
}

impl From<Shutdown> for PollError {
    fn from(_: Shutdown) -> Self {
        PollError(PollErrorKind::Shutdown)
    }
}

impl Error for AssociationError {}
impl Error for TaskError {}
impl Error for PollError {}
impl Error for CommandError {}
impl Error for TimeSyncError {}

#[cfg(test)]
mod test {
    use super::*;
    use crate::app::variations::Variation;
    use crate::app::QualifierCode;

    #[test]
    fn command_status_accessor_returns_status_only_for_bad_status() {
        let status_err: CommandError =
            CommandResponseError::BadStatus(CommandStatus::NotSupported).into();
        assert_eq!(
            status_err.command_status(),
            Some(CommandStatus::NotSupported)
        );

        let mismatch: CommandError = CommandResponseError::HeaderCountMismatch.into();
        assert_eq!(mismatch.command_status(), None);

        let task_err: CommandError = TaskErrorKind::ResponseTimeout.into();
        assert_eq!(task_err.command_status(), None);
    }

    #[test]
    fn task_error_words_parse_failures_by_cause() {
        let unsupported = TaskError::from(ObjectParseError::UnknownGroupVariation(1, 9));
        assert_eq!(
            unsupported.to_string(),
            "unsupported response content: unknown group/variation: g1v9"
        );

        let either = TaskError::from(ObjectParseError::InvalidQualifierForVariation(
            Variation::Group111(5),
            QualifierCode::Range8,
        ));
        assert_eq!(
            either.to_string(),
            "unsupported or invalid response content: Group111(5) may not be used with the qualifier: 1-byte start/stop (0x00)"
        );

        let malformed = TaskError::from(ObjectParseError::InsufficientBytes);
        assert_eq!(
            malformed.to_string(),
            "malformed response: insufficient bytes for object header"
        );
    }
}
