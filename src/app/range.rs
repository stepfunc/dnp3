use crate::app::header::FixedSizeVariation;
use crate::util::cursor::ReadCursor;

pub struct RangeMarker<'a> {
    start: u16,
    count: usize,
    data: &'a [u8],
}

impl<'a> RangeMarker<'a> {
    pub fn new(start: u16, count: usize, data: &'a [u8]) -> Self {
        Self { start, count, data }
    }
}

impl<'a> RangeMarker<'a> {
    pub fn iter<T>(&self) -> RangeIterator<'a, T>
    where
        T: FixedSizeVariation,
    {
        RangeIterator {
            index: self.start,
            count: self.count,
            cursor: ReadCursor::new(self.data),
            phantom: std::marker::PhantomData {},
        }
    }
}

pub struct RangeIterator<'a, T> {
    index: u16,
    count: usize,
    cursor: ReadCursor<'a>,
    phantom: std::marker::PhantomData<T>,
}

impl<'a, T> RangeIterator<'a, T> {
    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl<'a, T> Iterator for RangeIterator<'a, T>
where
    T: FixedSizeVariation,
{
    type Item = (T, u16);

    fn next(&mut self) -> Option<Self::Item> {
        match T::parse(&mut self.cursor) {
            Ok(x) => {
                let idx = self.index;
                self.index += 1;
                Some((x, idx))
            }
            Err(_) => None,
        }
    }
}
