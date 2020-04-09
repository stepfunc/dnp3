use std::fmt::Formatter;

pub trait Bitfield {
    const BIT_0: u8 = 0b0000_0001;
    const BIT_1: u8 = 0b0000_0010;
    const BIT_2: u8 = 0b0000_0100;
    const BIT_3: u8 = 0b0000_1000;
    const BIT_4: u8 = 0b0001_0000;
    const BIT_5: u8 = 0b0010_0000;
    const BIT_6: u8 = 0b0100_0000;
    const BIT_7: u8 = 0b1000_0000;

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
        self & Self::BIT_0 != 0
    }

    fn bit_1(self) -> bool {
        self & Self::BIT_1 != 0
    }

    fn bit_2(self) -> bool {
        self & Self::BIT_2 != 0
    }

    fn bit_3(self) -> bool {
        self & Self::BIT_3 != 0
    }

    fn bit_4(self) -> bool {
        self & Self::BIT_4 != 0
    }

    fn bit_5(self) -> bool {
        self & Self::BIT_5 != 0
    }

    fn bit_6(self) -> bool {
        self & Self::BIT_6 != 0
    }

    fn bit_7(self) -> bool {
        self & Self::BIT_7 != 0
    }
}

pub fn format_bitfield(
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
    write!(f, "{}: [", name)?;
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
