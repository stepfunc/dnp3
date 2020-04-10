use crate::app::gen::variations::fixed::*;
use crate::app::parse::traits::Index;

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

pub enum PrefixedCommandHeader<T>
where
    T: Index,
{
    G12V1(Vec<(Group12Var1, T)>),
    G41V1(Vec<(Group41Var1, T)>),
    G41V2(Vec<(Group41Var2, T)>),
    G41V3(Vec<(Group41Var3, T)>),
    G41V4(Vec<(Group41Var3, T)>),
}

pub enum CommandHeader {
    U8(PrefixedCommandHeader<u8>),
    U16(PrefixedCommandHeader<u16>),
}
