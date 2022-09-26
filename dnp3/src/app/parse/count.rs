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
        // this cannot overflow b/c SIZE is [0, 255] and count is [0, 65535]
        let num_bytes = T::SIZE as usize * count as usize;
        Ok(Self::new(count as usize, cursor.read_bytes(num_bytes)?))
    }

    pub(crate) fn single(&self) -> Option<T> {
        if self.count != 1 {
            return None;
        }

        self.iter().next()
    }

    pub(crate) fn new(count: usize, data: &'a [u8]) -> Self {
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

    fn next(&mut self) -> Option<Self::Item> {
        match T::read(&mut self.cursor) {
            Ok(x) => {
                self.remaining = self.remaining.saturating_sub(1);
                Some(x)
            }
            Err(_) => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}
