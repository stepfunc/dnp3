use crate::util::cursor::*;

pub trait FixedSize
where
    Self: Sized,
{
    const SIZE: u8;

    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError>;
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError>;
}

pub trait Index {
    fn widen_to_u16(self) -> u16;
}

impl FixedSize for u8 {
    const SIZE: u8 = 1;

    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        cursor.read_u8()
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(*self)
    }
}

impl FixedSize for u16 {
    const SIZE: u8 = 2;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        cursor.read_u16_le()
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u16_le(*self)
    }
}

impl Index for u8 {
    fn widen_to_u16(self) -> u16 {
        self as u16
    }
}

impl Index for u16 {
    fn widen_to_u16(self) -> u16 {
        self
    }
}
