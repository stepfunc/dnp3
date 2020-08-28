use crate::app::enums::{CommandStatus, OpType};
use crate::app::types::ControlCode;
use crate::app::variations::{Group12Var1, Group41Var1, Group41Var2, Group41Var3, Group41Var4};

impl Group12Var1 {
    /// construct a `Group12Var1` instance. The status field is automatically set to `CommandStatus::Success`
    pub fn new(code: ControlCode, count: u8, on_time: u32, off_time: u32) -> Self {
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
    pub fn new(value: i32) -> Self {
        Self {
            value,
            status: CommandStatus::Success,
        }
    }
}

impl Group41Var2 {
    /// construct a `Group41Var2` instance. The status field is automatically set to `CommandStatus::Success`
    pub fn new(value: i16) -> Self {
        Self {
            value,
            status: CommandStatus::Success,
        }
    }
}

impl Group41Var3 {
    /// construct a `Group41Var3` instance. The status field is automatically set to `CommandStatus::Success`
    pub fn new(value: f32) -> Self {
        Self {
            value,
            status: CommandStatus::Success,
        }
    }
}

impl Group41Var4 {
    /// construct a `Group41Var4` instance. The status field is automatically set to `CommandStatus::Success`
    pub fn new(value: f64) -> Self {
        Self {
            value,
            status: CommandStatus::Success,
        }
    }
}
