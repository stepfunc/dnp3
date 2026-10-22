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
        let count = usize::from(stop)
            .checked_sub(start.into())
            .and_then(|n| n.checked_add(1))
            .ok_or(InvalidRange { start, stop })?;

        Ok(Self { start, count })
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
        let num_bytes = usize::from(T::SIZE)
            .checked_mul(range.count)
            .ok_or(ReadError)?;
        Ok(Self::new(range, cursor.read_bytes(num_bytes)?))
    }

    pub(crate) fn empty() -> Self {
        Self::new(Range::empty(), &[])
    }

    fn new(range: Range, data: &'a [u8]) -> Self {
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

    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
    fn next(&mut self) -> Option<Self::Item> {
        self.remaining = self.remaining.checked_sub(1)?;
        let x = T::read(&mut self.cursor).ok()?;
        let idx = self.index;
        self.index = self.index.saturating_add(1);
        Some((x, idx))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}
