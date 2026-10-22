use crate::app::parse::traits::FixedSize;

use scursor::{ReadCursor, ReadError};

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct CountSequence<'a, T>
where
    T: FixedSize + Copy + Clone,
{
    count: usize,
    data: &'a [u8],
    phantom: std::marker::PhantomData<T>,
}

impl<'a, T> CountSequence<'a, T>
where
    T: FixedSize + Copy + Clone,
{
    pub(crate) fn parse(
        count: u16,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<CountSequence<'a, T>, ReadError> {
        let count = usize::from(count);
        let num_bytes = usize::from(T::SIZE).checked_mul(count).ok_or(ReadError)?;
        Ok(Self::new(count, cursor.read_bytes(num_bytes)?))
    }

    pub(crate) fn empty() -> Self {
        Self::new(0, &[])
    }

    pub(crate) fn single(&self) -> Option<T> {
        if self.count != 1 {
            return None;
        }

        self.iter().next()
    }

    fn new(count: usize, data: &'a [u8]) -> Self {
        Self {
            count,
            data,
            phantom: std::marker::PhantomData {},
        }
    }

    pub(crate) fn iter(&self) -> CountIterator<'a, T> {
        CountIterator {
            remaining: self.count,
            cursor: ReadCursor::new(self.data),
            phantom: std::marker::PhantomData {},
        }
    }
}

pub(crate) struct CountIterator<'a, T> {
    cursor: ReadCursor<'a>,
    remaining: usize,
    phantom: std::marker::PhantomData<T>,
}

impl<T> Iterator for CountIterator<'_, T>
where
    T: FixedSize,
{
    type Item = T;

    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
    fn next(&mut self) -> Option<Self::Item> {
        self.remaining = self.remaining.checked_sub(1)?;
        T::read(&mut self.cursor).ok()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_exactly_count_items() {
        let data = [0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0xFF];
        let mut cursor = ReadCursor::new(&data);
        let seq = CountSequence::<u16>::parse(3, &mut cursor).unwrap();
        assert_eq!(cursor.remaining(), 1);
        let items: Vec<u16> = seq.iter().collect();
        assert_eq!(items, vec![1, 2, 3]);
    }

    #[test]
    fn size_hint_counts_down_and_iterator_stays_exhausted() {
        let data = [0x01, 0x00, 0x02, 0x00];
        let mut cursor = ReadCursor::new(&data);
        let seq = CountSequence::<u16>::parse(2, &mut cursor).unwrap();
        let mut iter = seq.iter();
        assert_eq!(iter.size_hint(), (2, Some(2)));
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.size_hint(), (1, Some(1)));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }

    #[test]
    fn zero_count_consumes_nothing_and_yields_nothing() {
        let data = [0xAA];
        let mut cursor = ReadCursor::new(&data);
        let seq = CountSequence::<u16>::parse(0, &mut cursor).unwrap();
        assert_eq!(cursor.remaining(), 1);
        assert_eq!(seq.iter().size_hint(), (0, Some(0)));
        assert_eq!(seq.iter().next(), None);
        assert_eq!(seq.single(), None);
    }

    #[test]
    fn single_requires_a_count_of_one() {
        let data = [0x01, 0x00, 0x02, 0x00];
        let seq = CountSequence::<u16>::parse(1, &mut ReadCursor::new(&data)).unwrap();
        assert_eq!(seq.single(), Some(1));
        let seq = CountSequence::<u16>::parse(2, &mut ReadCursor::new(&data)).unwrap();
        assert_eq!(seq.single(), None);
    }

    #[test]
    fn fails_when_data_is_short() {
        let data = [0x01, 0x00, 0x02];
        let mut cursor = ReadCursor::new(&data);
        CountSequence::<u16>::parse(2, &mut cursor).unwrap_err();
    }
}
