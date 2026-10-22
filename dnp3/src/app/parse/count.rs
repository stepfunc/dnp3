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
