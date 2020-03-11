use super::function::Function;

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
    pub fn from(byte: u8) -> ControlField {
        ControlField {
            func: Function::from(byte & constants::MASK_FUNC_OR_PRM),
            master: (byte & constants::MASK_DIR) != 0,
            fcb: (byte & constants::MASK_FCB) != 0,
            fcv: (byte & constants::MASK_FCV) != 0,
        }
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
