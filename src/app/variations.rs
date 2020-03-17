use crate::app::header::FixedSizeVariation;
use crate::util::cursor::{ReadCursor, ReadError};

#[derive(Debug, PartialEq)]
pub struct Group2Var1 {
    pub flags: u8,
}

impl FixedSizeVariation for Group2Var1 {
    const SIZE: u8 = 1;
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(Group2Var1 {
            flags: cursor.read_u8()?,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct Group2Var2 {
    pub flags: u8,
    pub time: u64,
}

impl FixedSizeVariation for Group2Var2 {
    const SIZE: u8 = 7;
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(Group2Var2 {
            flags: cursor.read_u8()?,
            time: cursor.read_u48_le()?,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct Group2Var3 {
    pub flags: u8,
    pub time: u16,
}

impl FixedSizeVariation for Group2Var3 {
    const SIZE: u8 = 3;
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(Group2Var3 {
            flags: cursor.read_u8()?,
            time: cursor.read_u16_le()?,
        })
    }
}
