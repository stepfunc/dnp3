use std::fmt::Display;

use crate::app::variations::Variation;
use crate::app::QualifierCode;

use scursor::*;

pub(crate) trait FixedSize: Copy {
    const SIZE: u8;

    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError>;
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError>;
}

pub(crate) trait Index: FixedSize + PartialEq + Display {
    fn zero() -> Self;
    fn one() -> Self;
    /// the next index, or `None` past the type's maximum
    fn next(self) -> Option<Self>;
    fn widen_to_u16(self) -> u16;

    const COUNT_AND_PREFIX_QUALIFIER: QualifierCode;
    const RANGE_QUALIFIER: QualifierCode;
    const LIMITED_COUNT_QUALIFIER: QualifierCode;
}

pub(crate) trait FixedSizeVariation: FixedSize + PartialEq + Display {
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
    fn one() -> Self {
        1
    }
    fn next(self) -> Option<Self> {
        self.checked_add(1)
    }
    fn widen_to_u16(self) -> u16 {
        self.into()
    }

    const COUNT_AND_PREFIX_QUALIFIER: QualifierCode = QualifierCode::CountAndPrefix8;
    const RANGE_QUALIFIER: QualifierCode = QualifierCode::Range8;
    const LIMITED_COUNT_QUALIFIER: QualifierCode = QualifierCode::Count8;
}

impl Index for u16 {
    fn zero() -> Self {
        0
    }
    fn one() -> Self {
        1
    }
    fn next(self) -> Option<Self> {
        self.checked_add(1)
    }
    fn widen_to_u16(self) -> u16 {
        self
    }

    const COUNT_AND_PREFIX_QUALIFIER: QualifierCode = QualifierCode::CountAndPrefix16;
    const RANGE_QUALIFIER: QualifierCode = QualifierCode::Range16;
    const LIMITED_COUNT_QUALIFIER: QualifierCode = QualifierCode::Count16;
}
