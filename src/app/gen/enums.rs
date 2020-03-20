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

/// Used in conjunction with the TCC field to specify a control operation
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum OpType {
    Nul,
    PulseOn,
    PulseOff,
    LatchOn,
    LatchOff,
    /// captures any value not defined in the enumeration
    Unknown(u8),
}

impl OpType {
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
}

/// An enumeration of result codes received from an outstation in response to command request.
/// These correspond to those defined in the DNP3 standard
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CommandStatus {
    /// command was accepted, initiated, or queued
    Success,
    /// command timed out before completing
    Timeout,
    /// command requires being selected before operate, configuration issue
    NoSelect,
    /// bad control code or timing values
    FormatError,
    /// command is not implemented
    NotSupported,
    /// command is all ready in progress or its all ready in that mode
    AlreadyActive,
    /// something is stopping the command, often a local/remote interlock
    HardwareError,
    /// the function governed by the control is in local only control
    Local,
    /// the command has been done too often and has been throttled
    TooManyOps,
    /// the command was rejected because the device denied it or an RTU intercepted it
    NotAuthorized,
    /// command not accepted because it was prevented or inhibited by a local automation process, such as interlocking logic or synchrocheck
    AutomationInhibit,
    /// command not accepted because the device cannot process any more activities than are presently in progress
    ProcessingLimited,
    /// command not accepted because the value is outside the acceptable range permitted for this point
    OutOfRange,
    /// command not accepted because the outstation is forwarding the request to another downstream device which reported LOCAL
    DownstreamLocal,
    /// command not accepted because the outstation has already completed the requested operation
    AlreadyComplete,
    /// command not accepted because the requested function is specifically blocked at the outstation
    Blocked,
    /// command not accepted because the operation was cancelled
    Canceled,
    /// command not accepted because another master is communicating with the outstation and has exclusive rights to operate this control point
    BlockedOtherMaster,
    /// command not accepted because the outstation is forwarding the request to another downstream device which cannot be reached or is otherwise incapable of performing the request
    DownstreamFail,
    /// (deprecated) indicates the outstation shall not issue or perform the control operation
    NonParticipating,
    /// captures any value not defined in the enumeration
    Unknown(u8),
}

impl CommandStatus {
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
}

/// application object header types
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum QualifierCode {
    Range8,
    Range16,
    AllObjects,
    Count8,
    Count16,
    CountAndPrefix8,
    CountAndPrefix16,
    FreeFormat16,
}

impl QualifierCode {
    pub fn from(x: u8) -> Option<Self> {
        match x {
            0x0 => Some(QualifierCode::Range8),
            0x1 => Some(QualifierCode::Range16),
            0x6 => Some(QualifierCode::AllObjects),
            0x7 => Some(QualifierCode::Count8),
            0x8 => Some(QualifierCode::Count16),
            0x17 => Some(QualifierCode::CountAndPrefix8),
            0x28 => Some(QualifierCode::CountAndPrefix16),
            0x5B => Some(QualifierCode::FreeFormat16),
            _ => None,
        }
    }
}

/// Application layer function code enumeration
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FunctionCode {
    /// Master sends this to an outstation to confirm the receipt of an Application Layer fragment
    Confirm,
    /// Outstation shall return the data specified by the objects in the request
    Read,
    /// Outstation shall store the data specified by the objects in the request
    Write,
    /// Outstation shall select (or arm) the output points specified by the objects in the request in preparation for a subsequent operate command
    Select,
    /// Outstation shall activate the output points selected (or armed) by a previous select function code command
    Operate,
    /// Outstation shall immediately actuate the output points specified by the objects in the request
    DirectOperate,
    /// Same as DirectOperate but outstation shall not send a response
    DirectOperateNoResponse,
    /// Outstation shall copy the point data values specified by the objects in the request to a separate freeze buffer
    ImmediateFreeze,
    /// Same as ImmediateFreeze but outstation shall not send a response
    ImmediateFreezeNoResponse,
    /// Outstation shall copy the point data values specified by the objects in the request into a separate freeze buffer and then clear the values
    FreezeClear,
    /// Same as FreezeClear but outstation shall not send a response
    FreezeClearNoResponse,
    /// Outstation shall copy the point data values specified by the objects in the request to a separate freeze buffer at the time and/or time intervals specified in a special time data information object
    FreezeAtTime,
    /// Same as FreezeAtTime but outstation shall not send a response
    FreezeAtTimeNoResponse,
    /// Outstation shall perform a complete reset of all hardware and software in the device
    ColdRestart,
    /// Outstation shall reset only portions of the device
    WarmRestart,
    /// Obsolete-Do not use for new designs
    InitializeData,
    /// Outstation shall place the applications specified by the objects in the request into the ready to run state
    InitializeApplication,
    /// Outstation shall start running the applications specified by the objects in the request
    StartApplication,
    /// Outstation shall stop running the applications specified by the objects in the request
    StopApplication,
    /// This code is deprecated-Do not use for new designs
    SaveConfiguration,
    /// Enables outstation to initiate unsolicited responses from points specified by the objects in the request
    EnabledUnsolicited,
    /// Prevents outstation from initiating unsolicited responses from points specified by the objects in the request
    DisableUnsolicited,
    /// Outstation shall assign the events generated by the points specified by the objects in the request to one of the classes
    AssignClass,
    /// Outstation shall report the time it takes to process and initiate the transmission of its response
    DelayMeasure,
    /// Outstation shall save the time when the last octet of this message is received
    RecordCurrentTime,
    /// Outstation shall open a file
    OpenFile,
    /// Outstation shall close a file
    CloseFile,
    /// Outstation shall delete a file
    DeleteFile,
    /// Outstation shall retrieve information about a file
    GetFileInfo,
    /// Outstation shall return a file authentication key
    AuthenticateFile,
    /// Outstation shall abort a file transfer operation
    AbortFile,
    /// Master shall interpret this fragment as an Application Layer response to an ApplicationLayer request
    Response,
    /// Master shall interpret this fragment as an unsolicited response that was not prompted by an explicit request
    UnsolicitedResponse,
}

impl FunctionCode {
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
}
