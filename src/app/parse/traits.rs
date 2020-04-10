use crate::app::gen::enums::QualifierCode;
use crate::app::gen::variations::variation::Variation;
use crate::util::cursor::*;
use std::fmt::Display;

pub trait FixedSize
where
    Self: Sized,
{
    const SIZE: u8;

    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError>;
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError>;
}

pub trait Index: FixedSize + PartialEq + Display {
    fn zero() -> Self;
    fn increment(&mut self);
    fn widen_to_u16(self) -> u16;
    fn count_and_prefix_qualifier() -> QualifierCode;
    fn write_at(self, pos: usize, cursor: &mut WriteCursor) -> Result<(), WriteError>;
}

pub trait FixedSizeVariation: FixedSize + PartialEq + Display {
    const VARIATION: Variation;
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
    fn zero() -> Self {
        0
    }

    fn increment(&mut self) {
        *self += 1;
    }
    fn widen_to_u16(self) -> u16 {
        self as u16
    }
    fn count_and_prefix_qualifier() -> QualifierCode {
        QualifierCode::CountAndPrefix8
    }
    fn write_at(self, pos: usize, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8_at(self, pos)
    }
}

impl Index for u16 {
    fn zero() -> Self {
        0
    }
    fn increment(&mut self) {
        *self += 1;
    }
    fn widen_to_u16(self) -> u16 {
        self
    }
    fn count_and_prefix_qualifier() -> QualifierCode {
        QualifierCode::CountAndPrefix16
    }
    fn write_at(self, pos: usize, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u16_le_at(self, pos)
    }
}
