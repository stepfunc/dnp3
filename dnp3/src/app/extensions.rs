use crate::app::enums::{CommandStatus, OpType};
use crate::app::flags::Flags;
use crate::app::measurement::*;
use crate::app::types::ControlCode;
use crate::app::variations::{Group12Var1, Group41Var1, Group41Var2, Group41Var3, Group41Var4};
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

impl WireFlags for Binary {
    fn get_wire_flags(&self) -> u8 {
        self.flags.with_bits_set_to(BIT_7, self.value).value
    }
}

impl WireFlags for DoubleBitBinary {
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

impl WireFlags for Analog {
    fn get_wire_flags(&self) -> u8 {
        self.flags.value
    }
}

impl WireFlags for AnalogOutputStatus {
    fn get_wire_flags(&self) -> u8 {
        self.flags.value
    }
}

impl AnalogConversions for Analog {
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
