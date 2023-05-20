//  _   _         ______    _ _ _   _             _ _ _
// | \ | |       |  ____|  | (_) | (_)           | | | |
// |  \| | ___   | |__   __| |_| |_ _ _ __   __ _| | | |
// | . ` |/ _ \  |  __| / _` | | __| | '_ \ / _` | | | |
// | |\  | (_) | | |___| (_| | | |_| | | | | (_| |_|_|_|
// |_| \_|\___/  |______\__,_|_|\__|_|_| |_|\__, (_|_|_)
//                                           __/ |
//                                          |___/
//
// This file is auto-generated. Do not edit manually
//

use scursor::{WriteCursor, WriteError};

/// Field is used in conjunction with the `OpType` field to specify a control operation
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum TripCloseCode {
    ///  not specified (value == 0)
    Nul,
    ///  close output (value == 1)
    Close,
    ///  trip output (value == 2)
    Trip,
    ///  reserved for future use (value == 3)
    Reserved,
    /// captures any value not defined in the enumeration
    Unknown(u8),
}

impl TripCloseCode {
    /// create the enum from the underlying value
    pub fn from(x: u8) -> Self {
        match x {
            0 => TripCloseCode::Nul,
            1 => TripCloseCode::Close,
            2 => TripCloseCode::Trip,
            3 => TripCloseCode::Reserved,
            _ => TripCloseCode::Unknown(x),
        }
    }

    /// convert the enum to its underlying value
    pub fn as_u8(self) -> u8 {
        match self {
            TripCloseCode::Nul => 0,
            TripCloseCode::Close => 1,
            TripCloseCode::Trip => 2,
            TripCloseCode::Reserved => 3,
            TripCloseCode::Unknown(x) => x,
        }
    }
}

/// Field used in conjunction with the `TCC` field to specify a control operation
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum OpType {
    ///  not specified (value == 0)
    Nul,
    ///  pulse the output on (value == 1)
    PulseOn,
    ///  pulse the output off (value == 2)
    PulseOff,
    ///  latch the output on (value == 3)
    LatchOn,
    ///  latch the output off (value == 4)
    LatchOff,
    /// captures any value not defined in the enumeration
    Unknown(u8),
}

impl OpType {
    /// create the enum from the underlying value
    pub fn from(x: u8) -> Self {
        match x {
            0 => OpType::Nul,
            1 => OpType::PulseOn,
            2 => OpType::PulseOff,
            3 => OpType::LatchOn,
            4 => OpType::LatchOff,
            _ => OpType::Unknown(x),
        }
    }

    /// convert the enum to its underlying value
    pub fn as_u8(self) -> u8 {
        match self {
            OpType::Nul => 0,
            OpType::PulseOn => 1,
            OpType::PulseOff => 2,
            OpType::LatchOn => 3,
            OpType::LatchOff => 4,
            OpType::Unknown(x) => x,
        }
    }
}

/// Enumeration received from an outstation in response to command request
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum CommandStatus {
    ///  command was accepted, initiated, or queued (value == 0)
    Success,
    ///  command timed out before completing (value == 1)
    Timeout,
    ///  command requires being selected before operate, configuration issue (value == 2)
    NoSelect,
    ///  bad control code or timing values (value == 3)
    FormatError,
    ///  command is not implemented (value == 4)
    NotSupported,
    ///  command is all ready in progress or its all ready in that mode (value == 5)
    AlreadyActive,
    ///  something is stopping the command, often a local/remote interlock (value == 6)
    HardwareError,
    ///  the function governed by the control is in local only control (value == 7)
    Local,
    ///  the command has been done too often and has been throttled (value == 8)
    TooManyOps,
    ///  the command was rejected because the device denied it or an RTU intercepted it (value == 9)
    NotAuthorized,
    ///  command not accepted because it was prevented or inhibited by a local automation process, such as interlocking logic or synchrocheck (value == 10)
    AutomationInhibit,
    ///  command not accepted because the device cannot process any more activities than are presently in progress (value == 11)
    ProcessingLimited,
    ///  command not accepted because the value is outside the acceptable range permitted for this point (value == 12)
    OutOfRange,
    ///  command not accepted because the outstation is forwarding the request to another downstream device which reported LOCAL (value == 13)
    DownstreamLocal,
    ///  command not accepted because the outstation has already completed the requested operation (value == 14)
    AlreadyComplete,
    ///  command not accepted because the requested function is specifically blocked at the outstation (value == 15)
    Blocked,
    ///  command not accepted because the operation was cancelled (value == 16)
    Canceled,
    ///  command not accepted because another master is communicating with the outstation and has exclusive rights to operate this control point (value == 17)
    BlockedOtherMaster,
    ///  command not accepted because the outstation is forwarding the request to another downstream device which cannot be reached or is otherwise incapable of performing the request (value == 18)
    DownstreamFail,
    ///  (deprecated) indicates the outstation shall not issue or perform the control operation (value == 126)
    NonParticipating,
    /// captures any value not defined in the enumeration
    Unknown(u8),
}

impl CommandStatus {
    /// create the enum from the underlying value
    pub fn from(x: u8) -> Self {
        match x {
            0 => CommandStatus::Success,
            1 => CommandStatus::Timeout,
            2 => CommandStatus::NoSelect,
            3 => CommandStatus::FormatError,
            4 => CommandStatus::NotSupported,
            5 => CommandStatus::AlreadyActive,
            6 => CommandStatus::HardwareError,
            7 => CommandStatus::Local,
            8 => CommandStatus::TooManyOps,
            9 => CommandStatus::NotAuthorized,
            10 => CommandStatus::AutomationInhibit,
            11 => CommandStatus::ProcessingLimited,
            12 => CommandStatus::OutOfRange,
            13 => CommandStatus::DownstreamLocal,
            14 => CommandStatus::AlreadyComplete,
            15 => CommandStatus::Blocked,
            16 => CommandStatus::Canceled,
            17 => CommandStatus::BlockedOtherMaster,
            18 => CommandStatus::DownstreamFail,
            126 => CommandStatus::NonParticipating,
            _ => CommandStatus::Unknown(x),
        }
    }

    /// convert the enum to its underlying value
    pub fn as_u8(self) -> u8 {
        match self {
            CommandStatus::Success => 0,
            CommandStatus::Timeout => 1,
            CommandStatus::NoSelect => 2,
            CommandStatus::FormatError => 3,
            CommandStatus::NotSupported => 4,
            CommandStatus::AlreadyActive => 5,
            CommandStatus::HardwareError => 6,
            CommandStatus::Local => 7,
            CommandStatus::TooManyOps => 8,
            CommandStatus::NotAuthorized => 9,
            CommandStatus::AutomationInhibit => 10,
            CommandStatus::ProcessingLimited => 11,
            CommandStatus::OutOfRange => 12,
            CommandStatus::DownstreamLocal => 13,
            CommandStatus::AlreadyComplete => 14,
            CommandStatus::Blocked => 15,
            CommandStatus::Canceled => 16,
            CommandStatus::BlockedOtherMaster => 17,
            CommandStatus::DownstreamFail => 18,
            CommandStatus::NonParticipating => 126,
            CommandStatus::Unknown(x) => x,
        }
    }

    pub(crate) fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.as_u8())
    }
}
