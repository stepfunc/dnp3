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

use crate::util::cursor::{WriteCursor, WriteError};
use std::fmt::Formatter;

/// Field is used in conjunction with the `OpType` field to specify a control operation
#[derive(Copy, Clone, Debug, PartialEq)]
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
#[derive(Copy, Clone, Debug, PartialEq)]
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
#[derive(Copy, Clone, Debug, PartialEq)]
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

    pub(crate) fn is_success(self) -> bool {
        self == CommandStatus::Success
    }
    
    pub(crate) fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.as_u8())
    }

    pub(crate) fn first_error(&self, other: Self) -> Self {
        if self.is_success() { other } else { *self }
    }
}

/// Application object header types
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum QualifierCode {
    ///  8-bit start stop (value == 0x00)
    Range8,
    ///  16-bit start stop (value == 0x01)
    Range16,
    ///  all objects (value == 0x06)
    AllObjects,
    ///  8-bit count (value == 0x07)
    Count8,
    ///  16-bit count (value == 0x08)
    Count16,
    ///  8-bit count and prefix (value == 0x17)
    CountAndPrefix8,
    ///  16-bit count and prefix (value == 0x28)
    CountAndPrefix16,
    ///  16-bit free format (value == 0x5B)
    FreeFormat16,
}

impl QualifierCode {
    /// try to create the enum from the underlying value, returning None
    /// if the specified value is undefined
    pub fn from(x: u8) -> Option<Self> {
        match x {
            0x00 => Some(QualifierCode::Range8),
            0x01 => Some(QualifierCode::Range16),
            0x06 => Some(QualifierCode::AllObjects),
            0x07 => Some(QualifierCode::Count8),
            0x08 => Some(QualifierCode::Count16),
            0x17 => Some(QualifierCode::CountAndPrefix8),
            0x28 => Some(QualifierCode::CountAndPrefix16),
            0x5B => Some(QualifierCode::FreeFormat16),
            _ => None,
        }
    }
    
    /// convert the enum to its underlying value
    pub fn as_u8(self) -> u8 {
        match self {
            QualifierCode::Range8 => 0x00,
            QualifierCode::Range16 => 0x01,
            QualifierCode::AllObjects => 0x06,
            QualifierCode::Count8 => 0x07,
            QualifierCode::Count16 => 0x08,
            QualifierCode::CountAndPrefix8 => 0x17,
            QualifierCode::CountAndPrefix16 => 0x28,
            QualifierCode::FreeFormat16 => 0x5B,
        }
    }
    
    pub(crate) fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.as_u8())
    }
}

impl std::fmt::Display for QualifierCode {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            QualifierCode::Range8 => f.write_str("8-bit start stop (value == 0x00)"),
            QualifierCode::Range16 => f.write_str("16-bit start stop (value == 0x01)"),
            QualifierCode::AllObjects => f.write_str("all objects (value == 0x06)"),
            QualifierCode::Count8 => f.write_str("8-bit count (value == 0x07)"),
            QualifierCode::Count16 => f.write_str("16-bit count (value == 0x08)"),
            QualifierCode::CountAndPrefix8 => f.write_str("8-bit count and prefix (value == 0x17)"),
            QualifierCode::CountAndPrefix16 => f.write_str("16-bit count and prefix (value == 0x28)"),
            QualifierCode::FreeFormat16 => f.write_str("16-bit free format (value == 0x5B)"),
        }
    }
}

/// Application layer function code enumeration
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FunctionCode {
    ///  Master sends this to an outstation to confirm the receipt of an Application Layer fragment (value == 0)
    Confirm,
    ///  Outstation shall return the data specified by the objects in the request (value == 1)
    Read,
    ///  Outstation shall store the data specified by the objects in the request (value == 2)
    Write,
    ///  Outstation shall select (or arm) the output points specified by the objects in the request in preparation for a subsequent operate command (value == 3)
    Select,
    ///  Outstation shall activate the output points selected (or armed) by a previous select function code command (value == 4)
    Operate,
    ///  Outstation shall immediately actuate the output points specified by the objects in the request (value == 5)
    DirectOperate,
    ///  Same as DirectOperate but outstation shall not send a response (value == 6)
    DirectOperateNoResponse,
    ///  Outstation shall copy the point data values specified by the objects in the request to a separate freeze buffer (value == 7)
    ImmediateFreeze,
    ///  Same as ImmediateFreeze but outstation shall not send a response (value == 8)
    ImmediateFreezeNoResponse,
    ///  Outstation shall copy the point data values specified by the objects in the request into a separate freeze buffer and then clear the values (value == 9)
    FreezeClear,
    ///  Same as FreezeClear but outstation shall not send a response (value == 10)
    FreezeClearNoResponse,
    ///  Outstation shall copy the point data values specified by the objects in the request to a separate freeze buffer at the time and/or time intervals specified in a special time data information object (value == 11)
    FreezeAtTime,
    ///  Same as FreezeAtTime but outstation shall not send a response (value == 12)
    FreezeAtTimeNoResponse,
    ///  Outstation shall perform a complete reset of all hardware and software in the device (value == 13)
    ColdRestart,
    ///  Outstation shall reset only portions of the device (value == 14)
    WarmRestart,
    ///  Obsolete-Do not use for new designs (value == 15)
    InitializeData,
    ///  Outstation shall place the applications specified by the objects in the request into the ready to run state (value == 16)
    InitializeApplication,
    ///  Outstation shall start running the applications specified by the objects in the request (value == 17)
    StartApplication,
    ///  Outstation shall stop running the applications specified by the objects in the request (value == 18)
    StopApplication,
    ///  This code is deprecated-Do not use for new designs (value == 19)
    SaveConfiguration,
    ///  Enables outstation to initiate unsolicited responses from points specified by the objects in the request (value == 20)
    EnabledUnsolicited,
    ///  Prevents outstation from initiating unsolicited responses from points specified by the objects in the request (value == 21)
    DisableUnsolicited,
    ///  Outstation shall assign the events generated by the points specified by the objects in the request to one of the classes (value == 22)
    AssignClass,
    ///  Outstation shall report the time it takes to process and initiate the transmission of its response (value == 23)
    DelayMeasure,
    ///  Outstation shall save the time when the last octet of this message is received (value == 24)
    RecordCurrentTime,
    ///  Outstation shall open a file (value == 25)
    OpenFile,
    ///  Outstation shall close a file (value == 26)
    CloseFile,
    ///  Outstation shall delete a file (value == 27)
    DeleteFile,
    ///  Outstation shall retrieve information about a file (value == 28)
    GetFileInfo,
    ///  Outstation shall return a file authentication key (value == 29)
    AuthenticateFile,
    ///  Outstation shall abort a file transfer operation (value == 30)
    AbortFile,
    ///  Master shall interpret this fragment as an Application Layer response to an ApplicationLayer request (value == 129)
    Response,
    ///  Master shall interpret this fragment as an unsolicited response that was not prompted by an explicit request (value == 130)
    UnsolicitedResponse,
}

#[derive(Copy, Clone)]
pub(crate) enum FunctionType {
    Request,
    Response,
    Confirm,
}

#[derive(Copy, Clone)]
pub(crate) struct FunctionInfo {
    pub(crate) function_type: FunctionType,
    pub(crate) objects_allowed: bool,
}

impl FunctionInfo {

    const fn new(function_type: FunctionType, objects_allowed: bool) -> Self {
        Self { function_type, objects_allowed }
    }

    pub(crate) const fn request_with_objects() -> Self {
        Self { function_type: FunctionType::Request, objects_allowed: true }
    }

    pub(crate) const fn request_by_function_only() -> Self {
        Self { function_type: FunctionType::Request, objects_allowed: false }
    }

    pub(crate) const fn response() -> Self {
        Self { function_type: FunctionType::Response, objects_allowed: true }
    }

    pub(crate) const fn confirm() -> Self {
        Self { function_type: FunctionType::Confirm, objects_allowed: false }
    }
}

impl FunctionCode {
    /// try to create the enum from the underlying value, returning None
    /// if the specified value is undefined
    pub fn from(x: u8) -> Option<Self> {
        match x {
            0 => Some(FunctionCode::Confirm),
            1 => Some(FunctionCode::Read),
            2 => Some(FunctionCode::Write),
            3 => Some(FunctionCode::Select),
            4 => Some(FunctionCode::Operate),
            5 => Some(FunctionCode::DirectOperate),
            6 => Some(FunctionCode::DirectOperateNoResponse),
            7 => Some(FunctionCode::ImmediateFreeze),
            8 => Some(FunctionCode::ImmediateFreezeNoResponse),
            9 => Some(FunctionCode::FreezeClear),
            10 => Some(FunctionCode::FreezeClearNoResponse),
            11 => Some(FunctionCode::FreezeAtTime),
            12 => Some(FunctionCode::FreezeAtTimeNoResponse),
            13 => Some(FunctionCode::ColdRestart),
            14 => Some(FunctionCode::WarmRestart),
            15 => Some(FunctionCode::InitializeData),
            16 => Some(FunctionCode::InitializeApplication),
            17 => Some(FunctionCode::StartApplication),
            18 => Some(FunctionCode::StopApplication),
            19 => Some(FunctionCode::SaveConfiguration),
            20 => Some(FunctionCode::EnabledUnsolicited),
            21 => Some(FunctionCode::DisableUnsolicited),
            22 => Some(FunctionCode::AssignClass),
            23 => Some(FunctionCode::DelayMeasure),
            24 => Some(FunctionCode::RecordCurrentTime),
            25 => Some(FunctionCode::OpenFile),
            26 => Some(FunctionCode::CloseFile),
            27 => Some(FunctionCode::DeleteFile),
            28 => Some(FunctionCode::GetFileInfo),
            29 => Some(FunctionCode::AuthenticateFile),
            30 => Some(FunctionCode::AbortFile),
            129 => Some(FunctionCode::Response),
            130 => Some(FunctionCode::UnsolicitedResponse),
            _ => None,
        }
    }
    
    /// convert the enum to its underlying value
    pub fn as_u8(self) -> u8 {
        match self {
            FunctionCode::Confirm => 0,
            FunctionCode::Read => 1,
            FunctionCode::Write => 2,
            FunctionCode::Select => 3,
            FunctionCode::Operate => 4,
            FunctionCode::DirectOperate => 5,
            FunctionCode::DirectOperateNoResponse => 6,
            FunctionCode::ImmediateFreeze => 7,
            FunctionCode::ImmediateFreezeNoResponse => 8,
            FunctionCode::FreezeClear => 9,
            FunctionCode::FreezeClearNoResponse => 10,
            FunctionCode::FreezeAtTime => 11,
            FunctionCode::FreezeAtTimeNoResponse => 12,
            FunctionCode::ColdRestart => 13,
            FunctionCode::WarmRestart => 14,
            FunctionCode::InitializeData => 15,
            FunctionCode::InitializeApplication => 16,
            FunctionCode::StartApplication => 17,
            FunctionCode::StopApplication => 18,
            FunctionCode::SaveConfiguration => 19,
            FunctionCode::EnabledUnsolicited => 20,
            FunctionCode::DisableUnsolicited => 21,
            FunctionCode::AssignClass => 22,
            FunctionCode::DelayMeasure => 23,
            FunctionCode::RecordCurrentTime => 24,
            FunctionCode::OpenFile => 25,
            FunctionCode::CloseFile => 26,
            FunctionCode::DeleteFile => 27,
            FunctionCode::GetFileInfo => 28,
            FunctionCode::AuthenticateFile => 29,
            FunctionCode::AbortFile => 30,
            FunctionCode::Response => 129,
            FunctionCode::UnsolicitedResponse => 130,
        }
    }
    
    pub(crate) fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.as_u8())
    }

    pub(crate) fn get_function_info(&self) -> FunctionInfo {
        match self {
            // confirm
            FunctionCode::Confirm => FunctionInfo::confirm(),
            // requests that contain object headers
            FunctionCode::Read => FunctionInfo::request_with_objects(),
            FunctionCode::Write => FunctionInfo::request_with_objects(),
            FunctionCode::Select => FunctionInfo::request_with_objects(),
            FunctionCode::Operate => FunctionInfo::request_with_objects(),
            FunctionCode::DirectOperate => FunctionInfo::request_with_objects(),
            FunctionCode::DirectOperateNoResponse => FunctionInfo::request_with_objects(),
            FunctionCode::ImmediateFreeze => FunctionInfo::request_with_objects(),
            FunctionCode::ImmediateFreezeNoResponse => FunctionInfo::request_with_objects(),
            FunctionCode::FreezeClear => FunctionInfo::request_with_objects(),
            FunctionCode::FreezeClearNoResponse => FunctionInfo::request_with_objects(),
            FunctionCode::FreezeAtTime => FunctionInfo::request_with_objects(),
            FunctionCode::FreezeAtTimeNoResponse => FunctionInfo::request_with_objects(),
            FunctionCode::InitializeApplication => FunctionInfo::request_with_objects(),
            FunctionCode::StartApplication => FunctionInfo::request_with_objects(),
            FunctionCode::StopApplication => FunctionInfo::request_with_objects(),
            FunctionCode::EnabledUnsolicited => FunctionInfo::request_with_objects(),
            FunctionCode::DisableUnsolicited => FunctionInfo::request_with_objects(),
            FunctionCode::AssignClass => FunctionInfo::request_with_objects(),
            FunctionCode::OpenFile => FunctionInfo::request_with_objects(),
            FunctionCode::CloseFile => FunctionInfo::request_with_objects(),
            FunctionCode::DeleteFile => FunctionInfo::request_with_objects(),
            FunctionCode::GetFileInfo => FunctionInfo::request_with_objects(),
            FunctionCode::AuthenticateFile => FunctionInfo::request_with_objects(),
            FunctionCode::AbortFile => FunctionInfo::request_with_objects(),
            // requests that never have object headers
            FunctionCode::ColdRestart => FunctionInfo::request_by_function_only(),
            FunctionCode::WarmRestart => FunctionInfo::request_by_function_only(),
            FunctionCode::InitializeData => FunctionInfo::request_by_function_only(),
            FunctionCode::DelayMeasure => FunctionInfo::request_by_function_only(),
            FunctionCode::RecordCurrentTime => FunctionInfo::request_by_function_only(),
            FunctionCode::SaveConfiguration => FunctionInfo::request_by_function_only(),
            // responses
            FunctionCode::Response => FunctionInfo::response(),
            FunctionCode::UnsolicitedResponse => FunctionInfo::response(),
        }
    }
}

