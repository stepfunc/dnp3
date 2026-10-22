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
    const MASKS: [BitMask; 8] = [
        bits::BIT_0,
        bits::BIT_1,
        bits::BIT_2,
        bits::BIT_3,
        bits::BIT_4,
        bits::BIT_5,
        bits::BIT_6,
        bits::BIT_7,
    ];

    let mut prev = false;
    write!(f, "{name}: [")?;
    for (mask, name) in MASKS.iter().zip(names) {
        if value & mask.value != 0 {
            if prev {
                f.write_str(", ")?;
            }
            f.write_str(name)?;
            prev = true;
        }
    }

    f.write_str("]")
}

#[cfg(test)]
mod tests {
    use crate::app::{Iin1, Iin2};

    #[test]
    fn bitfield_lists_the_names_of_the_set_bits_from_bit_0() {
        assert_eq!(Iin1::new(0).to_string(), "iin1: []");
        assert_eq!(Iin1::new(0b0100_0000).to_string(), "iin1: [DEVICE_TROUBLE]");
        assert_eq!(
            Iin1::new(0b1000_0101).to_string(),
            "iin1: [BROADCAST, CLASS_2_EVENTS, DEVICE_RESTART]"
        );
        assert_eq!(
            Iin2::new(0xFF).to_string(),
            "iin2: [NO_FUNC_CODE_SUPPORT, OBJECT_UNKNOWN, PARAMETER_ERROR, EVENT_BUFFER_OVERFLOW, \
             ALREADY_EXECUTING, CONFIG_CORRUPT, RESERVED_2, RESERVED_1]"
        );
    }
}
