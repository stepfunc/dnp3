use crate::app::control::*;
use crate::app::measurement::*;
use crate::app::{FunctionCode, QualifierCode};
use crate::util::bit::bits::{BIT_6, BIT_7};

impl Group12Var1 {
    /// construct a `Group12Var1` instance. The status field is automatically set to `CommandStatus::Success`
    pub const fn new(code: ControlCode, count: u8, on_time: u32, off_time: u32) -> Self {
        Self {
            code,
            count,
            on_time,
            off_time,
            status: CommandStatus::Success,
        }
    }

    /// construct a `Group12Var1` instance from the `ControlCode`. Other fields are set to the following defaults:
    /// * count = 1
    /// * on_time = 1000
    /// * off_time = 1000
    /// * status = `CommandStatus::Success`
    pub fn from_code(code: ControlCode) -> Self {
        Self {
            code,
            count: 1,
            on_time: 1000,
            off_time: 1000,
            status: CommandStatus::Success,
        }
    }

    /// construct a `Group12Var1` instance from an `OpType`. Other fields are set to the following defaults:
    /// * count = 1
    /// * on_time = 1000
    /// * off_time = 1000
    /// * status = `CommandStatus::Success`
    pub fn from_op_type(op: OpType) -> Self {
        Self {
            code: ControlCode::from_op_type(op),
            count: 1,
            on_time: 1000,
            off_time: 1000,
            status: CommandStatus::Success,
        }
    }
}

impl Group41Var1 {
    /// construct a `Group41Var1` instance. The status field is automatically set to `CommandStatus::Success`
    pub const fn new(value: i32) -> Self {
        Self {
            value,
            status: CommandStatus::Success,
        }
    }
}

impl Group41Var2 {
    /// construct a `Group41Var2` instance. The status field is automatically set to `CommandStatus::Success`
    pub const fn new(value: i16) -> Self {
        Self {
            value,
            status: CommandStatus::Success,
        }
    }
}

impl Group41Var3 {
    /// construct a `Group41Var3` instance. The status field is automatically set to `CommandStatus::Success`
    pub const fn new(value: f32) -> Self {
        Self {
            value,
            status: CommandStatus::Success,
        }
    }
}

impl Group41Var4 {
    /// construct a `Group41Var4` instance. The status field is automatically set to `CommandStatus::Success`
    pub const fn new(value: f64) -> Self {
        Self {
            value,
            status: CommandStatus::Success,
        }
    }
}

impl WireFlags for BinaryInput {
    fn get_wire_flags(&self) -> u8 {
        self.flags.with_bits_set_to(BIT_7, self.value).value
    }
}

impl WireFlags for DoubleBitBinaryInput {
    fn get_wire_flags(&self) -> u8 {
        let pair = self.value.to_bit_pair();
        self.flags
            .with_bits_set_to(BIT_7, pair.high)
            .with_bits_set_to(BIT_6, pair.low)
            .value
    }
}

impl WireFlags for BinaryOutputStatus {
    fn get_wire_flags(&self) -> u8 {
        self.flags.with_bits_set_to(BIT_7, self.value).value
    }
}

impl WireFlags for Counter {
    fn get_wire_flags(&self) -> u8 {
        self.flags.value
    }
}

impl WireFlags for FrozenCounter {
    fn get_wire_flags(&self) -> u8 {
        self.flags.value
    }
}

impl WireFlags for AnalogInput {
    fn get_wire_flags(&self) -> u8 {
        self.flags.value
    }
}

impl WireFlags for AnalogOutputStatus {
    fn get_wire_flags(&self) -> u8 {
        self.flags.value
    }
}

impl AnalogConversions for AnalogInput {
    fn get_value(&self) -> f64 {
        self.value
    }

    fn get_flags(&self) -> Flags {
        self.flags
    }
}

impl AnalogConversions for FrozenAnalogInput {
    fn get_value(&self) -> f64 {
        self.value
    }

    fn get_flags(&self) -> Flags {
        self.flags
    }
}

impl AnalogConversions for AnalogOutputStatus {
    fn get_value(&self) -> f64 {
        self.value
    }

    fn get_flags(&self) -> Flags {
        self.flags
    }
}

impl CommandStatus {
    pub(crate) fn is_success(self) -> bool {
        self == CommandStatus::Success
    }

    pub(crate) fn first_error(&self, other: Self) -> Self {
        if self.is_success() {
            other
        } else {
            *self
        }
    }
}

impl std::fmt::Display for QualifierCode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            QualifierCode::Range8 => f.write_str("8-bit start stop (value == 0x00)"),
            QualifierCode::Range16 => f.write_str("16-bit start stop (value == 0x01)"),
            QualifierCode::AllObjects => f.write_str("all objects (value == 0x06)"),
            QualifierCode::Count8 => f.write_str("8-bit count (value == 0x07)"),
            QualifierCode::Count16 => f.write_str("16-bit count (value == 0x08)"),
            QualifierCode::CountAndPrefix8 => f.write_str("8-bit count and prefix (value == 0x17)"),
            QualifierCode::CountAndPrefix16 => {
                f.write_str("16-bit count and prefix (value == 0x28)")
            }
            QualifierCode::FreeFormat16 => f.write_str("16-bit free format (value == 0x5B)"),
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) struct FunctionInfo {
    pub(crate) objects_allowed: bool,
}

impl FunctionInfo {
    pub(crate) const fn request_with_objects() -> Self {
        Self {
            objects_allowed: true,
        }
    }

    pub(crate) const fn request_by_function_only() -> Self {
        Self {
            objects_allowed: false,
        }
    }

    pub(crate) const fn response() -> Self {
        Self {
            objects_allowed: true,
        }
    }

    pub(crate) const fn confirm() -> Self {
        Self {
            objects_allowed: false,
        }
    }
}

impl FunctionCode {
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
            FunctionCode::EnableUnsolicited => FunctionInfo::request_with_objects(),
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
