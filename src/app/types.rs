use crate::app::gen::enums::{OpType, TripCloseCode};

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
            tcc: TripCloseCode::from(x & Self::TCC_MASK >> 6),
            clear: x & Self::CR_MASK != 0,
            queue: x & Self::QU_MASK != 0,
            op_type: OpType::from(x & Self::OP_MASK),
        }
    }
}
