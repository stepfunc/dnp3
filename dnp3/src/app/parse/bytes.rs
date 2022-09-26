use crate::app::parse::traits::FixedSize;
use crate::app::parse_error::ObjectParseError;

use scursor::ReadCursor;

/// Wrapper around an underlying u8 slice
#[derive(Debug, PartialEq)]
pub(crate) struct Bytes<'a> {
    /// underlying slice
    pub(crate) value: &'a [u8],
}

impl<'a> Bytes<'a> {
    pub(crate) fn new(value: &'a [u8]) -> Self {
        Self { value }
    }
}

impl std::fmt::Display for Bytes<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.value.len() <= 3 {
            return write!(f, "{:02X?}", self.value);
        }

        if let Some(s) = self.value.get(0..3) {
            return write!(f, "length = {}, {:02X?} ...", self.value.len(), s);
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct RangedBytesSequence<'a> {
    bytes: &'a [u8],
    index: u16,
    size: usize,
    count: usize,
}

#[derive(Debug)]
pub(crate) struct RangedBytesIterator<'a> {
    cursor: ReadCursor<'a>,
    index: u16,
    size: usize,
    remaining: usize,
}

#[derive(Debug)]
pub(crate) struct PrefixedBytesSequence<'a, T>
where
    T: FixedSize,
{
    bytes: &'a [u8],
    size: usize,
    count: usize,
    phantom: std::marker::PhantomData<T>,
}

pub(crate) struct PrefixedBytesIterator<'a, T>
where
    T: FixedSize,
{
    cursor: ReadCursor<'a>,
    size: usize,
    remaining: usize,
    phantom: std::marker::PhantomData<T>,
}

impl<'a> RangedBytesSequence<'a> {
    pub(crate) fn parse(
        variation: u8,
        index: u16,
        count: usize,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<Self, ObjectParseError> {
        if variation == 0 {
            return Err(ObjectParseError::ZeroLengthOctetData);
        }

        Ok(RangedBytesSequence {
            bytes: cursor.read_bytes(variation as usize * count)?,
            index,
            size: variation as usize,
            count,
        })
    }

    pub(crate) fn iter(&self) -> RangedBytesIterator<'a> {
        RangedBytesIterator {
            cursor: ReadCursor::new(self.bytes),
            index: self.index,
            size: self.size,
            remaining: self.count,
        }
    }
}

impl<'a, T> PrefixedBytesSequence<'a, T>
where
    T: FixedSize,
{
    pub(crate) fn parse(
        variation: u8,
        count: u16,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<Self, ObjectParseError> {
        if variation == 0 {
            return Err(ObjectParseError::ZeroLengthOctetData);
        }

        let size = (variation as usize + T::SIZE as usize) * count as usize;

        Ok(PrefixedBytesSequence {
            bytes: cursor.read_bytes(size)?,
            size: variation as usize,
            count: count as usize,
            phantom: std::marker::PhantomData {},
        })
    }

    pub(crate) fn iter(&self) -> PrefixedBytesIterator<'a, T> {
        PrefixedBytesIterator {
            cursor: ReadCursor::new(self.bytes),
            size: self.size,
            remaining: self.count,
            phantom: std::marker::PhantomData {},
        }
    }
}

impl<'a> Iterator for RangedBytesIterator<'a> {
    type Item = (&'a [u8], u16);

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.read_bytes(self.size).ok().map(|b| {
            let index = self.index;
            self.index = self.index.saturating_add(1);
            self.remaining = self.remaining.saturating_sub(1);
            (b, index)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, T> Iterator for PrefixedBytesIterator<'a, T>
where
    T: FixedSize,
{
    type Item = (&'a [u8], T);

    fn next(&mut self) -> Option<Self::Item> {
        let index = match T::read(&mut self.cursor) {
            Ok(x) => x,
            _ => return None,
        };
        let bytes = match self.cursor.read_bytes(self.size) {
            Ok(x) => x,
            _ => return None,
        };

        self.remaining -= 1;

        Some((bytes, index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bytes_formats_as_expected() {
        let short = Bytes::new(&[0x01, 0x02, 0x03]);
        let long = Bytes::new(&[0x01, 0x02, 0x03, 0x04]);

        assert_eq!(format!("{}", short), "[01, 02, 03]");
        assert_eq!(format!("{}", long), "length = 4, [01, 02, 03] ...");
    }
}
