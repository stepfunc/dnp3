use super::variations::*;
use crate::app::range::RangedSequence;
use crate::util::cursor::{ReadCursor, ReadError};

pub trait FixedSizeVariation
where
    Self: Sized,
{
    const SIZE: u8;

    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError>;
}

#[derive(Debug, PartialEq)]
pub enum RangedVariation<'a> {
    Group2Var0,
    Group2Var1(RangedSequence<'a, Group2Var1>),
    Group2Var2(RangedSequence<'a, Group2Var2>),
    Group2Var3(RangedSequence<'a, Group2Var3>),
}

#[derive(Debug, PartialEq)]
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

        let seq = RangedSequence::<Group2Var1>::new(1, &[0xAA, 0xBB]);

        let items: Vec<(Group2Var1, u16)> = seq.iter().collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], (Group2Var1 { flags: 0xAA }, 1));
        assert_eq!(items[1], (Group2Var1 { flags: 0xBB }, 2));
    }
}
