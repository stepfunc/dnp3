use crate::app::gen::variations::all::AllObjectsVariation;
use crate::app::gen::variations::count::CountVariation;
use crate::app::gen::variations::prefixed::PrefixedVariation;
use crate::app::gen::variations::ranged::RangedVariation;
use crate::util::cursor::{ReadCursor, ReadError};

pub trait FixedSize
where
    Self: Sized,
{
    const SIZE: u8;

    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError>;
}

impl FixedSize for u8 {
    const SIZE: u8 = 1;
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        cursor.read_u8()
    }
}

impl FixedSize for u16 {
    const SIZE: u8 = 2;
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        cursor.read_u16_le()
    }
}

#[derive(Debug, PartialEq)]
pub enum Header<'a> {
    AllObjects(AllObjectsVariation),
    OneByteStartStop(u8, u8, RangedVariation<'a>),
    TwoByteStartStop(u16, u16, RangedVariation<'a>),
    OneByteCount(u8, CountVariation<'a>),
    TwoByteCount(u16, CountVariation<'a>),
    OneByteCountAndPrefix(u8, PrefixedVariation<'a, u8>),
    TwoByteCountAndPrefix(u16, PrefixedVariation<'a, u16>),
}

#[cfg(test)]
mod test {
    use crate::app::gen::variations::fixed::Group2Var1;
    use crate::app::range::RangedSequence;

    #[test]
    fn iterates_over_g2v1() {
        let seq = RangedSequence::<Group2Var1>::new(1, &[0xAA, 0xBB]);

        let items: Vec<(Group2Var1, u16)> = seq.iter().collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], (Group2Var1 { flags: 0xAA }, 1));
        assert_eq!(items[1], (Group2Var1 { flags: 0xBB }, 2));
    }
}
