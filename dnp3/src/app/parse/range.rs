// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

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
    stop: u16,
    count: usize,
}

impl Range {
    /// construct a range in a way that ensures only a valid range can be obtained
    pub(crate) fn from(start: u16, stop: u16) -> Result<Self, InvalidRange> {
        let count = usize::from(stop)
            .checked_sub(start.into())
            .and_then(|n| n.checked_add(1))
            .ok_or(InvalidRange { start, stop })?;

        Ok(Self { start, stop, count })
    }

    pub(crate) fn get_start(&self) -> u16 {
        self.start
    }

    /// the last index of a range from [`Range::from`]
    pub(crate) fn get_stop(&self) -> u16 {
        self.stop
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_exactly_range_count_items_with_indices() {
        let data = [0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0xFF];
        let mut cursor = ReadCursor::new(&data);
        let seq = RangedSequence::<u16>::parse(Range::from(10, 12).unwrap(), &mut cursor).unwrap();
        assert_eq!(cursor.remaining(), 1);
        let items: Vec<(u16, u16)> = seq.iter().collect();
        assert_eq!(items, vec![(1, 10), (2, 11), (3, 12)]);
    }

    #[test]
    fn parses_at_max_index() {
        let data = [0x01, 0x00, 0x02, 0x00];
        let mut cursor = ReadCursor::new(&data);
        let seq =
            RangedSequence::<u16>::parse(Range::from(65534, 65535).unwrap(), &mut cursor).unwrap();
        assert!(cursor.is_empty());
        let items: Vec<(u16, u16)> = seq.iter().collect();
        assert_eq!(items, vec![(1, 65534), (2, 65535)]);
    }

    #[test]
    fn size_hint_counts_down_and_iterator_stays_exhausted() {
        let data = [0x01, 0x00, 0x02, 0x00];
        let mut cursor = ReadCursor::new(&data);
        let seq = RangedSequence::<u16>::parse(Range::from(0, 1).unwrap(), &mut cursor).unwrap();
        let mut iter = seq.iter();
        assert_eq!(iter.size_hint(), (2, Some(2)));
        assert_eq!(iter.next(), Some((1, 0)));
        assert_eq!(iter.size_hint(), (1, Some(1)));
        assert_eq!(iter.next(), Some((2, 1)));
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }

    #[test]
    fn fails_when_data_is_short() {
        let data = [0x01, 0x00, 0x02];
        let mut cursor = ReadCursor::new(&data);
        RangedSequence::<u16>::parse(Range::from(0, 1).unwrap(), &mut cursor).unwrap_err();
    }
}
