use crate::app::gen::variations::fixed::*;

#[derive(Copy, Clone)]
pub struct ClassScan {
    pub class1: bool,
    pub class2: bool,
    pub class3: bool,
    pub class0: bool,
}

impl ClassScan {
    pub fn new(class1: bool, class2: bool, class3: bool, class0: bool) -> Self {
        Self {
            class1,
            class2,
            class3,
            class0,
        }
    }

    pub fn class1() -> Self {
        Self::new(true, false, false, false)
    }

    pub fn class123() -> Self {
        Self::new(true, true, true, false)
    }

    pub fn integrity() -> Self {
        Self::new(true, true, true, true)
    }
}

pub enum CommandHeader {
    G12V1PrefixedU8(Vec<(Group12Var1, u8)>),
    G12V1Prefixed16(Vec<(Group12Var1, u16)>),
    G41V1PrefixedU8(Vec<(Group41Var1, u8)>),
    G41V1Prefixed16(Vec<(Group41Var1, u16)>),
    G41V2PrefixedU8(Vec<(Group41Var2, u8)>),
    G41V2PrefixedU16(Vec<(Group41Var2, u16)>),
    G41V3PrefixedU8(Vec<(Group41Var3, u8)>),
    G41V3PrefixedU16(Vec<(Group41Var3, u16)>),
    G41V4PrefixedU8(Vec<(Group41Var3, u8)>),
    G41V4PrefixedU16(Vec<(Group41Var3, u16)>),
}

pub struct CommandRequest {
    pub(crate) headers: Vec<CommandHeader>,
}

impl CommandRequest {
    pub fn new(headers: Vec<CommandHeader>) -> Self {
        Self { headers }
    }
}
