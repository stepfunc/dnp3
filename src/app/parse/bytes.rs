use crate::app::parse::header::FixedSize;
use crate::app::parse::parser::ParseError;
use crate::util::cursor::ReadCursor;

#[derive(Debug, PartialEq)]
pub struct Bytes<'a> {
    pub value: &'a [u8],
}

#[derive(Debug, PartialEq)]
pub struct RangedBytesSequence<'a> {
    bytes: &'a [u8],
    index: u16,
    size: usize,
}

#[derive(Debug, PartialEq)]
pub struct RangedBytesIterator<'a> {
    cursor: ReadCursor<'a>,
    index: u16,
    size: usize,
}

#[derive(Debug, PartialEq)]
pub struct PrefixedBytesSequence<'a, T>
where
    T: FixedSize,
{
    bytes: &'a [u8],
    size: usize,
    phantom: std::marker::PhantomData<T>,
}

pub struct PrefixedBytesIterator<'a, T>
where
    T: FixedSize,
{
    cursor: ReadCursor<'a>,
    size: usize,
    phantom: std::marker::PhantomData<T>,
}

impl<'a> Bytes<'a> {
    pub fn new(value: &'a [u8]) -> Self {
        Self { value }
    }
}

impl<'a> RangedBytesSequence<'a> {
    pub fn parse(
        variation: u8,
        index: u16,
        count: usize,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<Self, ParseError> {
        if variation == 0 {
            return Err(ParseError::ZeroLengthOctetData);
        }

        Ok(RangedBytesSequence {
            bytes: cursor.read_bytes(variation as usize * count)?,
            index,
            size: variation as usize,
        })
    }

    pub fn iter(&self) -> RangedBytesIterator<'a> {
        RangedBytesIterator {
            cursor: ReadCursor::new(self.bytes),
            index: self.index,
            size: self.size,
        }
    }
}

impl<'a, T> PrefixedBytesSequence<'a, T>
where
    T: FixedSize,
{
    pub fn parse(
        variation: u8,
        count: u16,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<Self, ParseError> {
        if variation == 0 {
            return Err(ParseError::ZeroLengthOctetData);
        }

        let size = (variation as usize + T::SIZE as usize) * count as usize;

        Ok(PrefixedBytesSequence {
            bytes: cursor.read_bytes(size)?,
            size: variation as usize,
            phantom: std::marker::PhantomData {},
        })
    }

    pub fn iter(&self) -> PrefixedBytesIterator<'a, T> {
        PrefixedBytesIterator {
            cursor: ReadCursor::new(self.bytes),
            size: self.size,
            phantom: std::marker::PhantomData {},
        }
    }
}

impl<'a> Iterator for RangedBytesIterator<'a> {
    type Item = (Bytes<'a>, u16);

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.read_bytes(self.size).ok().map(|b| {
            let index = self.index;
            self.index += 1;
            (Bytes::new(b), index)
        })
    }
}

impl<'a, T> Iterator for PrefixedBytesIterator<'a, T>
where
    T: FixedSize,
{
    type Item = (Bytes<'a>, T);

    fn next(&mut self) -> Option<Self::Item> {
        let bytes = match self.cursor.read_bytes(self.size) {
            Ok(x) => x,
            _ => return None,
        };
        let index = match T::parse(&mut self.cursor) {
            Ok(x) => x,
            _ => return None,
        };

        Some((Bytes::new(bytes), index))
    }
}
