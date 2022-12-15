use std::fmt::Formatter;
use std::ops::BitOr;

pub(crate) struct BitMask {
    pub(crate) value: u8,
}

impl BitOr<BitMask> for BitMask {
    type Output = BitMask;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value | rhs.value,
        }
    }
}

pub(crate) mod bits {
    use crate::util::bit::BitMask;

    pub(crate) const BIT_0: BitMask = BitMask { value: 0b0000_0001 };
    pub(crate) const BIT_1: BitMask = BitMask { value: 0b0000_0010 };
    pub(crate) const BIT_2: BitMask = BitMask { value: 0b0000_0100 };
    pub(crate) const BIT_3: BitMask = BitMask { value: 0b0000_1000 };
    pub(crate) const BIT_4: BitMask = BitMask { value: 0b0001_0000 };
    pub(crate) const BIT_5: BitMask = BitMask { value: 0b0010_0000 };
    pub(crate) const BIT_6: BitMask = BitMask { value: 0b0100_0000 };
    pub(crate) const BIT_7: BitMask = BitMask { value: 0b1000_0000 };
}

pub(crate) trait Bitfield {
    fn bit_0(self) -> bool;
    fn bit_1(self) -> bool;
    fn bit_2(self) -> bool;
    fn bit_3(self) -> bool;
    fn bit_4(self) -> bool;
    fn bit_5(self) -> bool;
    fn bit_6(self) -> bool;
    fn bit_7(self) -> bool;
}

impl Bitfield for u8 {
    fn bit_0(self) -> bool {
        self & bits::BIT_0.value != 0
    }

    fn bit_1(self) -> bool {
        self & bits::BIT_1.value != 0
    }

    fn bit_2(self) -> bool {
        self & bits::BIT_2.value != 0
    }

    fn bit_3(self) -> bool {
        self & bits::BIT_3.value != 0
    }

    fn bit_4(self) -> bool {
        self & bits::BIT_4.value != 0
    }

    fn bit_5(self) -> bool {
        self & bits::BIT_5.value != 0
    }

    fn bit_6(self) -> bool {
        self & bits::BIT_6.value != 0
    }

    fn bit_7(self) -> bool {
        self & bits::BIT_7.value != 0
    }
}

pub(crate) fn format_bitfield(
    f: &mut Formatter,
    value: u8,
    name: &'static str,
    names: [&'static str; 8],
) -> std::fmt::Result {
    fn push(f: &mut Formatter, prev: bool, s: &'static str) -> std::fmt::Result {
        if prev {
            f.write_str(", ")?;
        }
        f.write_str(s)
    }

    let mut prev = false;
    write!(f, "{name}: [")?;
    if value.bit_0() {
        push(f, prev, names[0])?;
        prev = true;
    }
    if value.bit_1() {
        push(f, prev, names[1])?;
        prev = true;
    }
    if value.bit_2() {
        push(f, prev, names[2])?;
        prev = true;
    }
    if value.bit_3() {
        push(f, prev, names[3])?;
        prev = true;
    }
    if value.bit_4() {
        push(f, prev, names[4])?;
        prev = true;
    }
    if value.bit_5() {
        push(f, prev, names[5])?;
        prev = true;
    }
    if value.bit_6() {
        push(f, prev, names[6])?;
        prev = true;
    }
    if value.bit_7() {
        push(f, prev, names[7])?;
    }

    f.write_str("]")
}
