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
pub struct Ctrl {
    pub func: Function,
    pub master: bool,
    pub fcb: bool,
    pub fcv: bool,
}

impl Ctrl {
    pub fn from(byte: u8) -> Ctrl {
        Ctrl {
            func: Function::from(byte & constants::MASK_FUNC_OR_PRM),
            master: (byte & constants::MASK_DIR) != 0,
            fcb: (byte & constants::MASK_FCB) != 0,
            fcv: (byte & constants::MASK_FCV) != 0,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Header {
    pub ctrl: Ctrl,
    pub dest: u16,
    pub src: u16,
}

impl Header {
    pub fn from(ctrl: Ctrl, dest: u16, src: u16) -> Header {
        Header { ctrl, dest, src }
    }
}
