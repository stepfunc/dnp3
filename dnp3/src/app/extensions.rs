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

impl FunctionCode {
    /// whether a fragment with this function code may carry object headers
    pub(crate) fn objects_allowed(self) -> bool {
        match self {
            FunctionCode::Read
            | FunctionCode::Write
            | FunctionCode::Select
            | FunctionCode::Operate
            | FunctionCode::DirectOperate
            | FunctionCode::DirectOperateNoResponse
            | FunctionCode::ImmediateFreeze
            | FunctionCode::ImmediateFreezeNoResponse
            | FunctionCode::FreezeClear
            | FunctionCode::FreezeClearNoResponse
            | FunctionCode::FreezeAtTime
            | FunctionCode::FreezeAtTimeNoResponse
            | FunctionCode::InitializeApplication
            | FunctionCode::StartApplication
            | FunctionCode::StopApplication
            | FunctionCode::EnableUnsolicited
            | FunctionCode::DisableUnsolicited
            | FunctionCode::AssignClass
            | FunctionCode::OpenFile
            | FunctionCode::CloseFile
            | FunctionCode::DeleteFile
            | FunctionCode::GetFileInfo
            | FunctionCode::AuthenticateFile
            | FunctionCode::AbortFile
            | FunctionCode::Response
            | FunctionCode::UnsolicitedResponse => true,
            FunctionCode::Confirm
            | FunctionCode::ColdRestart
            | FunctionCode::WarmRestart
            | FunctionCode::InitializeData
            | FunctionCode::DelayMeasure
            | FunctionCode::RecordCurrentTime
            | FunctionCode::SaveConfiguration => false,
        }
    }
}
