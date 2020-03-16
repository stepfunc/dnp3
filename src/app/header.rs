use super::variations::*;
use crate::app::range::{RangeIterator, RangeMarker};
use crate::util::cursor::{ReadCursor, ReadError};

pub trait FixedSizeVariation
where
    Self: Sized,
{
    const SIZE: u8;
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError>;
}

#[repr(transparent)]
pub struct Group2Var1Seq<'a> {
    marker: RangeMarker<'a>,
}

impl<'a> Group2Var1Seq<'a> {
    pub fn iter(&self) -> RangeIterator<'a, Group2Var1> {
        self.marker.iter()
    }
}
#[repr(transparent)]
pub struct Group2Var2Seq<'a> {
    marker: RangeMarker<'a>,
}

impl<'a> Group2Var2Seq<'a> {
    pub fn iter(&self) -> RangeIterator<'a, Group2Var2> {
        self.marker.iter()
    }
}
#[repr(transparent)]
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
    //Group2Var3(Group2Var3Seq<'a>),
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

        println!("header: {}", std::mem::size_of::<Header>());
        println!("header[64]: {}", std::mem::size_of::<[Header; 128]>());

        let seq = Group2Var1Seq {
            marker: RangeMarker::new(1, &[0xAA, 0xBB]),
        };

        let items: Vec<(Group2Var1, u16)> = seq.iter().collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], (Group2Var1 { flags: 0xAA }, 1));
        assert_eq!(items[1], (Group2Var1 { flags: 0xBB }, 2));
    }
}
