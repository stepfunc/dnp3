use crate::app::header::FixedSize;
use crate::util::cursor::{ReadCursor, ReadError};

#[derive(Debug, PartialEq)]
pub struct CountSequence<'a, T>
where
    T: FixedSize,
{
    data: &'a [u8],
    phantom: std::marker::PhantomData<T>,
}

impl<'a, T> CountSequence<'a, T>
where
    T: FixedSize,
{
    pub fn parse(
        count: u16,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<CountSequence<'a, T>, ReadError> {
        // this cannot overflow b/c SIZE is [0, 255] and count is [0, 65535]
        let num_bytes = T::SIZE as usize * count as usize;
        Ok(Self::new(cursor.read_bytes(num_bytes)?))
    }

    pub fn empty() -> Self {
        Self::new(&[])
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            phantom: std::marker::PhantomData {},
        }
    }

    pub fn iter(&self) -> CountIterator<'a, T> {
        CountIterator {
            cursor: ReadCursor::new(self.data),
            phantom: std::marker::PhantomData {},
        }
    }
}

pub struct CountIterator<'a, T> {
    cursor: ReadCursor<'a>,
    phantom: std::marker::PhantomData<T>,
}

impl<'a, T> Iterator for CountIterator<'a, T>
where
    T: FixedSize,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match T::parse(&mut self.cursor) {
            Ok(x) => Some(x),
            Err(_) => None,
        }
    }
}
