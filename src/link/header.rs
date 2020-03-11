use super::function::Function;
use crate::link::header::constants::{MASK_DIR, MASK_FCB, MASK_FCV};

pub mod constants {
    pub const MASK_DIR: u8 = 0x80;
    pub const MASK_PRM: u8 = 0x40;
    pub const MASK_FCB: u8 = 0x20;
    pub const MASK_FCV: u8 = 0x10;
    pub const MASK_FUNC: u8 = 0x0F;
    pub const MASK_FUNC_OR_PRM: u8 = MASK_PRM | MASK_FUNC;
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct ControlField {
    pub func: Function,
    pub master: bool,
    pub fcb: bool,
    pub fcv: bool,
}

impl ControlField {
    pub fn new(master: bool, function: Function) -> Self {
        Self {
            func: function,
            master,
            fcb: false,
            fcv: false,
        }
    }

    pub fn from(byte: u8) -> ControlField {
        ControlField {
            func: Function::from(byte & constants::MASK_FUNC_OR_PRM),
            master: (byte & constants::MASK_DIR) != 0,
            fcb: (byte & constants::MASK_FCB) != 0,
            fcv: (byte & constants::MASK_FCV) != 0,
        }
    }

    pub fn to_u8(&self) -> u8 {
        let mut ret = 0;
        ret |= if self.master { MASK_DIR } else { 0 };
        // the PRM bit is part of the function code
        ret |= if self.fcb { MASK_FCB } else { 0 };
        ret |= if self.fcv { MASK_FCV } else { 0 };
        ret |= self.func.to_u8();
        ret
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Address {
    pub destination: u16,
    pub source: u16,
}

impl Address {
    pub fn new(destination: u16, source: u16) -> Self {
        Self {
            destination,
            source,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Header {
    pub control: ControlField,
    pub address: Address,
}

impl Header {
    pub fn new(control: ControlField, address: Address) -> Header {
        Header { control, address }
    }
}
