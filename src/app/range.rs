use crate::app::header::FixedSizeVariation;
use crate::util::cursor::ReadCursor;

pub struct RangeMarker<'a> {
    start: u16,
    data: &'a [u8],
}

impl<'a> RangeMarker<'a> {
    pub fn new(start: u16, data: &'a [u8]) -> Self {
        Self { start, data }
    }
}

impl<'a> RangeMarker<'a> {
    pub fn iter<T>(&self) -> RangeIterator<'a, T>
    where
        T: FixedSizeVariation,
    {
        RangeIterator {
            index: self.start,
            cursor: ReadCursor::new(self.data),
            phantom: std::marker::PhantomData {},
        }
    }
}

pub struct RangeIterator<'a, T> {
    index: u16,
    cursor: ReadCursor<'a>,
    phantom: std::marker::PhantomData<T>,
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
