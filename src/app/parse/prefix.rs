use crate::app::parse::traits::FixedSize;
use crate::util::cursor::{ReadCursor, ReadError, WriteCursor, WriteError};

#[derive(Debug, PartialEq)]
pub struct Prefix<I, V>
where
    I: FixedSize + std::fmt::Display,
    V: FixedSize,
{
    pub index: I,
    pub value: V,
}

impl<I, V> FixedSize for Prefix<I, V>
where
    I: FixedSize + std::fmt::Display,
    V: FixedSize,
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
