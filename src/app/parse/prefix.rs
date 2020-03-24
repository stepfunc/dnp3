use crate::app::parse::header::FixedSize;
use crate::util::cursor::{ReadCursor, ReadError};

#[derive(Debug, PartialEq)]
pub struct Prefix<I, V>
where
    I: FixedSize,
    V: FixedSize,
{
    pub index: I,
    pub value: V,
}

impl<I, V> FixedSize for Prefix<I, V>
where
    I: FixedSize,
    V: FixedSize,
{
    const SIZE: u8 = I::SIZE + V::SIZE;

    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(Prefix {
            index: I::parse(cursor)?,
            value: V::parse(cursor)?,
        })
    }
}
