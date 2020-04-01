use crate::app::gen::enums::{OpType, TripCloseCode};
use crate::app::gen::variations::gv::Variation;
use std::fmt::Formatter;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DoubleBit {
    Intermediate,
    DeterminedOff,
    DeterminedOn,
    Indeterminate,
}

impl DoubleBit {
    // the lowest two bits of this number
    pub fn from(x: u8) -> Self {
        match x & 0b0000_0011 {
            0b00 => DoubleBit::Intermediate,
            0b01 => DoubleBit::DeterminedOff,
            0b10 => DoubleBit::DeterminedOn,
            _ => DoubleBit::Indeterminate,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ControlCode {
    pub tcc: TripCloseCode,
    pub clear: bool,
    pub queue: bool,
    pub op_type: OpType,
}

impl ControlCode {
    const TCC_MASK: u8 = 0b1100_0000;
    const CR_MASK: u8 = 0b0010_0000;
    const QU_MASK: u8 = 0b0001_0000;
    const OP_MASK: u8 = 0b0000_1111;

    pub fn from(x: u8) -> Self {
        Self {
            tcc: TripCloseCode::from((x & Self::TCC_MASK) >> 6),
            clear: x & Self::CR_MASK != 0,
            queue: x & Self::QU_MASK != 0,
            op_type: OpType::from(x & Self::OP_MASK),
        }
    }
    pub fn as_u8(self) -> u8 {
        let mut x = 0;
        x |= self.tcc.as_u8() << 6;
        if self.clear {
            x |= Self::CR_MASK;
        }
        if self.queue {
            x |= Self::QU_MASK;
        }
        x |= self.op_type.as_u8();
        x
    }
}

impl std::fmt::Display for Variation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (g,v) = self.to_group_and_var();
        write!(f, "g{}v{}", g, v)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_control_code_round_trip(byte: u8, cc: ControlCode) {
        assert_eq!(cc.as_u8(), byte);
        assert_eq!(ControlCode::from(byte), cc)
    }

    #[test]
    fn correctly_converts_control_code_to_and_from_u8() {
        test_control_code_round_trip(
            0b10_1_1_0100,
            ControlCode {
                tcc: TripCloseCode::Trip,
                clear: true,
                queue: true,
                op_type: OpType::LatchOff,
            },
        );

        test_control_code_round_trip(
            0b10_0_1_0100,
            ControlCode {
                tcc: TripCloseCode::Trip,
                clear: false,
                queue: true,
                op_type: OpType::LatchOff,
            },
        );

        test_control_code_round_trip(
            0b10_1_0_0100,
            ControlCode {
                tcc: TripCloseCode::Trip,
                clear: true,
                queue: false,
                op_type: OpType::LatchOff,
            },
        );

        test_control_code_round_trip(
            0b11_0_0_0000,
            ControlCode {
                tcc: TripCloseCode::Reserved,
                clear: false,
                queue: false,
                op_type: OpType::Nul,
            },
        );
    }
}
