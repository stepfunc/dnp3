// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Internal bridge for `dnp3-ffi`. Not part of the public Rust API.

use crate::app::{ParseErrorCause, Shutdown};
use crate::master::error::{
    AssociationError, AssociationErrorKind, CommandError, CommandErrorKind, CommandResponseError,
    PollError, PollErrorKind, TaskError, TaskErrorKind, TimeSyncError, TimeSyncErrorKind,
};
use crate::master::file::{FileError, FileErrorKind};
use crate::outstation::database::{AttrDefError, AttrDefErrorKind};

/// Construct a [`Shutdown`] value for FFI promise drop handlers.
pub fn shutdown() -> Shutdown {
    Shutdown
}

/// Classification of [`TaskError`] for the FFI bindings.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TaskErrorClass {
    /// Too many user requests are queued.
    TooManyRequests,
    /// No active connection or link/transport failure.
    NoConnection,
    /// Response objects use an unsupported encoding.
    UnsupportedResponse,
    /// Response is malformed or unexpected.
    BadResponse,
    /// Response qualifier is unsupported or invalid.
    UnsupportedOrBadResponse,
    /// Response timed out.
    ResponseTimeout,
    /// Request could not be serialized into the transmit buffer.
    WriteError,
    /// Target association was removed or does not exist.
    AssociationRemoved,
    /// Master task was shut down.
    Shutdown,
    /// Request data could not be encoded.
    BadEncoding,
    /// Outstation rejected the request with IIN2 error bits.
    IinError,
    /// System time was not available.
    SystemTimeNotAvailable,
}

impl From<TaskErrorKind> for TaskErrorClass {
    fn from(err: TaskErrorKind) -> Self {
        match err {
            TaskErrorKind::TooManyRequests => Self::TooManyRequests,
            TaskErrorKind::Link(_)
            | TaskErrorKind::Transport
            | TaskErrorKind::NoConnection
            | TaskErrorKind::Disabled => Self::NoConnection,
            TaskErrorKind::MalformedResponse(err) => match err.cause() {
                ParseErrorCause::Unsupported => Self::UnsupportedResponse,
                ParseErrorCause::Invalid => Self::BadResponse,
                ParseErrorCause::UnsupportedOrInvalid => Self::UnsupportedOrBadResponse,
            },
            TaskErrorKind::UnexpectedResponseHeaders
            | TaskErrorKind::NonFinWithoutCon
            | TaskErrorKind::NeverReceivedFir
            | TaskErrorKind::UnexpectedFir
            | TaskErrorKind::MultiFragmentResponse => Self::BadResponse,
            TaskErrorKind::ResponseTimeout => Self::ResponseTimeout,
            TaskErrorKind::WriteError => Self::WriteError,
            TaskErrorKind::NoSuchAssociation(_) => Self::AssociationRemoved,
            TaskErrorKind::Shutdown => Self::Shutdown,
            TaskErrorKind::BadEncoding(_) => Self::BadEncoding,
            TaskErrorKind::RejectedByIin2(_) => Self::IinError,
            TaskErrorKind::SystemTimeNotAvailable => Self::SystemTimeNotAvailable,
        }
    }
}

impl From<TaskError> for TaskErrorClass {
    fn from(err: TaskError) -> Self {
        Self::from(err.0)
    }
}

/// Classification of [`CommandError`] for the FFI bindings.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CommandErrorClass {
    /// Outstation returned a non-success command status.
    BadStatus,
    /// Response headers or values did not echo the request.
    HeaderMismatch,
    /// Underlying task failure.
    Task(TaskErrorClass),
}

impl From<CommandError> for CommandErrorClass {
    fn from(err: CommandError) -> Self {
        match err.0 {
            CommandErrorKind::Task(task) => Self::Task(task.into()),
            CommandErrorKind::Response(CommandResponseError::BadStatus(_)) => Self::BadStatus,
            CommandErrorKind::Response(
                CommandResponseError::HeaderCountMismatch
                | CommandResponseError::HeaderTypeMismatch
                | CommandResponseError::ObjectCountMismatch
                | CommandResponseError::ObjectValueMismatch,
            ) => Self::HeaderMismatch,
        }
    }
}

/// Classification of [`TimeSyncError`] for the FFI bindings.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TimeSyncErrorClass {
    /// Detected a clock rollback.
    ClockRollback,
    /// Outstation time delay exceeded response delay.
    BadOutstationTimeDelay,
    /// Overflow in time calculation.
    Overflow,
    /// Outstation did not clear NEED_TIME.
    StillNeedsTime,
    /// Underlying task failure.
    Task(TaskErrorClass),
}

impl From<TimeSyncError> for TimeSyncErrorClass {
    fn from(err: TimeSyncError) -> Self {
        match err.0 {
            TimeSyncErrorKind::Task(task) => Self::Task(task.into()),
            TimeSyncErrorKind::ClockRollback => Self::ClockRollback,
            TimeSyncErrorKind::BadOutstationTimeDelay(_) => Self::BadOutstationTimeDelay,
            TimeSyncErrorKind::Overflow => Self::Overflow,
            TimeSyncErrorKind::StillNeedsTime => Self::StillNeedsTime,
        }
    }
}

/// Classification of [`FileError`] for the FFI bindings.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FileErrorClass {
    /// Outstation returned a non-success file status.
    BadStatus,
    /// Outstation denied permission.
    NoPermission,
    /// Received an unexpected block number.
    BadBlockNum,
    /// Transfer aborted by user.
    AbortByUser,
    /// Exceeded maximum configured length.
    MaxLengthExceeded,
    /// Returned file handle did not match request.
    WrongHandle,
    /// Underlying task failure.
    Task(TaskErrorClass),
}

impl From<FileError> for FileErrorClass {
    fn from(err: FileError) -> Self {
        match err.0 {
            FileErrorKind::BadStatus(_) => Self::BadStatus,
            FileErrorKind::NoPermission => Self::NoPermission,
            FileErrorKind::BadBlockNum => Self::BadBlockNum,
            FileErrorKind::AbortByUser => Self::AbortByUser,
            FileErrorKind::MaxLengthExceeded => Self::MaxLengthExceeded,
            FileErrorKind::WrongHandle => Self::WrongHandle,
            FileErrorKind::Task(task) => Self::Task(task.into()),
        }
    }
}

/// Classification of [`AssociationError`] for the FFI bindings.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AssociationErrorClass {
    /// Master task has shut down.
    Shutdown,
    /// Duplicate association address.
    DuplicateAddress,
    /// Wrong channel type for this operation.
    WrongChannelType,
}

impl From<AssociationError> for AssociationErrorClass {
    fn from(err: AssociationError) -> Self {
        match err.0 {
            AssociationErrorKind::Shutdown => Self::Shutdown,
            AssociationErrorKind::DuplicateAddress(_) => Self::DuplicateAddress,
            AssociationErrorKind::WrongChannelType { .. } => Self::WrongChannelType,
        }
    }
}

/// Classification of [`PollError`] for the FFI bindings.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PollErrorClass {
    /// Master task has shut down.
    Shutdown,
    /// Target association does not exist.
    NoSuchAssociation,
}

impl From<PollError> for PollErrorClass {
    fn from(err: PollError) -> Self {
        match err.0 {
            PollErrorKind::Shutdown => Self::Shutdown,
            PollErrorKind::NoSuchAssociation(_) => Self::NoSuchAssociation,
        }
    }
}

/// Classification of [`AttrDefError`] for the FFI bindings.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AttrDefErrorClass {
    /// Attribute is already defined.
    AlreadyDefined,
    /// Attribute type does not match expected type.
    BadType,
    /// Reserved variation cannot be defined.
    ReservedVariation,
    /// Attribute is not writable.
    NotWritable,
    /// Attribute value cannot be encoded.
    BadValue,
}

impl From<AttrDefError> for AttrDefErrorClass {
    fn from(err: AttrDefError) -> Self {
        match err.0 {
            AttrDefErrorKind::AlreadyDefined => Self::AlreadyDefined,
            AttrDefErrorKind::BadType(_) => Self::BadType,
            AttrDefErrorKind::ReservedVariation(_) => Self::ReservedVariation,
            AttrDefErrorKind::NotWritable(_, _) => Self::NotWritable,
            AttrDefErrorKind::BadValue(_) => Self::BadValue,
        }
    }
}

/// Classification of [`crate::tcp::tls::TlsError`] for the FFI bindings.
#[cfg(feature = "enable-tls")]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TlsErrorClass {
    /// The configured DNS name is invalid.
    InvalidDnsName,
    /// Any other TLS configuration error.
    Other,
}

#[cfg(feature = "enable-tls")]
impl From<crate::tcp::tls::TlsError> for TlsErrorClass {
    fn from(err: crate::tcp::tls::TlsError) -> Self {
        match err.0 {
            crate::tcp::tls::TlsErrorKind::InvalidDnsName => Self::InvalidDnsName,
            crate::tcp::tls::TlsErrorKind::Config(_) => Self::Other,
        }
    }
}
