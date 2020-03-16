use crate::util::cursor::{ReadCursor, ReadError};
use std::marker::PhantomData;

pub trait FixedSizeVariation
where
    Self: Sized,
{
    const SIZE: u8;
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError>;
}

#[derive(Debug, PartialEq)]
pub struct Group2Var1 {
    flags: u8,
}

impl FixedSizeVariation for Group2Var1 {
    const SIZE: u8 = 1;
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(Group2Var1 {
            flags: cursor.read_u8()?,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct Group2Var2 {
    flags: u8,
    time: u64,
}

impl FixedSizeVariation for Group2Var2 {
    const SIZE: u8 = 3; // TODO
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(Group2Var2 {
            flags: cursor.read_u8()?,
            time: cursor.read_u16_le()? as u64, // TODO - need u48
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct Group2Var3 {
    flags: u8,
    time: u16,
}

impl FixedSizeVariation for Group2Var3 {
    const SIZE: u8 = 3; // TODO

    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(Group2Var3 {
            flags: cursor.read_u8()?,
            time: cursor.read_u16_le()?,
        })
    }
}

struct RangeMarker<'a> {
    start: u16,
    count: usize,
    data: &'a [u8],
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
            phantom: PhantomData {},
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

pub struct Group2Var1Seq<'a> {
    marker: RangeMarker<'a>,
}

impl<'a> Group2Var1Seq<'a> {
    pub fn iter(&self) -> RangeIterator<'a, Group2Var1> {
        self.marker.iter()
    }
}

pub struct Group2Var2Seq<'a> {
    marker: RangeMarker<'a>,
}

impl<'a> Group2Var2Seq<'a> {
    pub fn iter(&self) -> RangeIterator<'a, Group2Var2> {
        self.marker.iter()
    }
}

pub struct Group2Var3Seq<'a> {
    marker: RangeMarker<'a>,
}

impl<'a> Group2Var3Seq<'a> {
    pub fn iter(&self) -> RangeIterator<'a, Group2Var3> {
        self.marker.iter()
    }
}

pub enum RangedVariation<'a> {
    Group2Var0,
    Group2Var1(Group2Var1Seq<'a>),
    Group2Var2(Group2Var2Seq<'a>),
    Group2Var3(Group2Var3Seq<'a>),
}

pub enum Header<'a> {
    OneByteStartStop(u8, u8, RangedVariation<'a>),
    TwoByteStartStop(u16, u16, RangedVariation<'a>),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn iterates_over_g2v1() {
        let seq = Group2Var1Seq {
            marker: RangeMarker {
                start: 1,
                count: 2,
                data: &[0xAA, 0xBB],
            },
        };

        let items: Vec<(Group2Var1, u16)> = seq.iter().collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], (Group2Var1 { flags: 0xAA }, 1));
        assert_eq!(items[1], (Group2Var1 { flags: 0xBB }, 2));

        seq.iter().len();
    }
}
