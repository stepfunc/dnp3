use crate::app::parse::traits::{FixedSize, FixedSizeVariation, Index};

use scursor::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Prefix<I, V>
where
    I: Index,
    V: FixedSizeVariation,
{
    pub(crate) index: I,
    pub(crate) value: V,
}

impl<I, V> Prefix<I, V>
where
    I: Index,
    V: FixedSizeVariation,
{
    pub(crate) fn equals(&self, other: &(V, I)) -> bool {
        self.index == other.1 && self.value == other.0
    }
}

impl<I, V> FixedSize for Prefix<I, V>
where
    I: Index,
    V: FixedSizeVariation,
{
    const SIZE: u8 = I::SIZE + V::SIZE;

    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(Prefix {
            index: I::read(cursor)?,
            value: V::read(cursor)?,
        })
    }

    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.index.write(cursor)?;
        self.value.write(cursor)
    }
}
