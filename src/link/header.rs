use super::function::Function;

pub(crate) mod constants {
    pub(crate) const MASK_DIR: u8 = 0x80;
    pub(crate) const MASK_PRM: u8 = 0x40;
    pub(crate) const MASK_FCB: u8 = 0x20;
    pub(crate) const MASK_FCV: u8 = 0x10;
    pub(crate) const MASK_FUNC: u8 = 0x0F;
    pub(crate) const MASK_FUNC_OR_PRM: u8 = MASK_PRM | MASK_FUNC;
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) struct ControlField {
    pub(crate) func: Function,
    pub(crate) master: bool,
    pub(crate) fcb: bool,
    pub(crate) fcv: bool,
}

impl ControlField {
    pub(crate) fn new(master: bool, function: Function) -> Self {
        Self {
            func: function,
            master,
            fcb: false,
            fcv: false,
        }
    }

    pub(crate) fn from(byte: u8) -> ControlField {
        ControlField {
            func: Function::from(byte & constants::MASK_FUNC_OR_PRM),
            master: (byte & constants::MASK_DIR) != 0,
            fcb: (byte & constants::MASK_FCB) != 0,
            fcv: (byte & constants::MASK_FCV) != 0,
        }
    }

    pub(crate) fn to_u8(self) -> u8 {
        let mut ret = 0;
        ret |= if self.master { constants::MASK_DIR } else { 0 };
        // the PRM bit is part of the function code
        ret |= if self.fcb { constants::MASK_FCB } else { 0 };
        ret |= if self.fcv { constants::MASK_FCV } else { 0 };
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
pub(crate) struct Header {
    pub(crate) control: ControlField,
    pub(crate) address: Address,
}

impl Header {
    pub(crate) fn new(control: ControlField, address: Address) -> Header {
        Header { control, address }
    }
}
