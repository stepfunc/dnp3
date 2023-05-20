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

/// Application object header types
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
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

/// Application layer function code enumeration
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
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
    EnableUnsolicited,
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
            20 => Some(FunctionCode::EnableUnsolicited),
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
            FunctionCode::EnableUnsolicited => 20,
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
}

