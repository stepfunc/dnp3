use crate::app::parse::traits::FixedSize;

use scursor::{ReadCursor, ReadError};

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct InvalidRange {
    pub(crate) start: u16,
    pub(crate) stop: u16,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Range {
    start: u16,
    count: usize,
}

impl Range {
    /// construct a range in a way that ensures only a valid range can be obtained
    pub(crate) fn from(start: u16, stop: u16) -> Result<Self, InvalidRange> {
        if stop < start {
            return Err(InvalidRange { start, stop });
        }

        Ok(Self {
            start,
            count: stop as usize - start as usize + 1,
        })
    }

    pub(crate) fn empty() -> Self {
        Self { start: 0, count: 0 }
    }

    pub(crate) fn get_start(&self) -> u16 {
        self.start
    }

    pub(crate) fn get_count(&self) -> usize {
        self.count
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct RangedSequence<'a, T>
where
    T: FixedSize,
{
    range: Range,
    data: &'a [u8],
    phantom: std::marker::PhantomData<T>,
}

impl<'a, T> RangedSequence<'a, T>
where
    T: FixedSize,
{
    pub(crate) fn parse(
        range: Range,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<RangedSequence<'a, T>, ReadError> {
        // this cannot overflow b/c SIZE is [0, 255] and count is [0, 65536]
        let num_bytes = T::SIZE as usize * range.count;
        Ok(Self::new(range, cursor.read_bytes(num_bytes)?))
    }

    pub(crate) fn empty() -> Self {
        Self::new(Range::empty(), &[])
    }

    pub(crate) fn new(range: Range, data: &'a [u8]) -> Self {
        Self {
            range,
            data,
            phantom: std::marker::PhantomData {},
        }
    }

    pub(crate) fn iter(&self) -> RangeIterator<'a, T> {
        RangeIterator {
            index: self.range.start,
            remaining: self.range.count,
            cursor: ReadCursor::new(self.data),
            phantom: std::marker::PhantomData {},
        }
    }
}

pub(crate) struct RangeIterator<'a, T> {
    index: u16,
    remaining: usize,
    cursor: ReadCursor<'a>,
    phantom: std::marker::PhantomData<T>,
}

impl<T> Iterator for RangeIterator<'_, T>
where
    T: FixedSize,
{
    type Item = (T, u16);

    fn next(&mut self) -> Option<Self::Item> {
        match T::read(&mut self.cursor) {
            Ok(x) => {
                let idx = self.index;
                self.index = self.index.saturating_add(1);
                self.remaining = self.remaining.saturating_sub(1);
                Some((x, idx))
            }
            Err(_) => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}
