use crate::util::cursor::{ReadCursor, ReadError};

pub trait FixedSize
where
    Self: Sized,
{
    const SIZE: u8;

    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError>;
}

impl FixedSize for u8 {
    const SIZE: u8 = 1;
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        cursor.read_u8()
    }
}

impl FixedSize for u16 {
    const SIZE: u8 = 2;
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        cursor.read_u16_le()
    }
}
