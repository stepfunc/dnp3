use crate::app::format::write::HeaderWriter;
use crate::app::gen::variations::fixed::*;
use crate::app::parse::traits::{FixedSize, Index};
use crate::util::cursor::WriteError;

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

pub enum PrefixedCommandHeader<I>
where
    I: Index + FixedSize,
{
    G12V1(Vec<(Group12Var1, I)>),
    G41V1(Vec<(Group41Var1, I)>),
    G41V2(Vec<(Group41Var2, I)>),
    G41V3(Vec<(Group41Var3, I)>),
    G41V4(Vec<(Group41Var3, I)>),
}

impl<I> PrefixedCommandHeader<I>
where
    I: Index + FixedSize,
{
    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match self {
            PrefixedCommandHeader::G12V1(items) => writer.write_prefixed_header(items.iter()),
            PrefixedCommandHeader::G41V1(items) => writer.write_prefixed_header(items.iter()),
            PrefixedCommandHeader::G41V2(items) => writer.write_prefixed_header(items.iter()),
            PrefixedCommandHeader::G41V3(items) => writer.write_prefixed_header(items.iter()),
            PrefixedCommandHeader::G41V4(items) => writer.write_prefixed_header(items.iter()),
        }
    }
}

pub enum CommandHeader {
    U8(PrefixedCommandHeader<u8>),
    U16(PrefixedCommandHeader<u16>),
}

impl CommandHeader {
    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match self {
            CommandHeader::U8(header) => header.write(writer),
            CommandHeader::U16(header) => header.write(writer),
        }
    }
}
