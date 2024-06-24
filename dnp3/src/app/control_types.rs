use crate::app::control_enums::{OpType, TripCloseCode};

/// Control code field used within g12v1
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct ControlCode {
    /// This field is used in conjunction with the `op_type` field to specify a control operation
    pub tcc: TripCloseCode,
    /// Support for this field is optional. When the clear bit is set, the device shall remove pending control commands for that
    /// index and stop any control operation that is in progress for that index. The indexed point shall go to the state that it
    /// would have if the command were allowed to complete normally.
    pub clear: bool,
    /// This field is obsolete. Masters shall always set this bit to 0. Outstations that receive a
    /// g12v1 object with this bit set shall reply with `CommandStatus::NotSupported`
    pub queue: bool,
    /// This field is used in conjunction with the `tcc` field to specify a control operation
    pub op_type: OpType,
}

impl ControlCode {
    const TCC_MASK: u8 = 0b1100_0000;
    const CR_MASK: u8 = 0b0010_0000;
    const QU_MASK: u8 = 0b0001_0000;
    const OP_MASK: u8 = 0b0000_1111;

    /// Create a `ControlCode` from its constituent parts. Initializes the `queue` field to false.
    pub fn new(tcc: TripCloseCode, op_type: OpType, clear: bool) -> Self {
        Self {
            tcc,
            clear,
            queue: false,
            op_type,
        }
    }

    /// Create a `ControlField` from an `OpType` only.
    ///
    /// `tcc` is set to Nul
    /// `clear` is set to false
    pub fn from_op_type(value: OpType) -> Self {
        Self::new(TripCloseCode::Nul, value, false)
    }

    /// Create a `ControlField` from a `TripCloseCode` and an `OpType`
    ///
    /// `clear` is set to false
    pub fn from_tcc_and_op_type(tcc: TripCloseCode, op_type: OpType) -> Self {
        Self::new(tcc, op_type, false)
    }

    pub(crate) fn from(x: u8) -> Self {
        Self {
            tcc: TripCloseCode::from((x & Self::TCC_MASK) >> 6),
            clear: x & Self::CR_MASK != 0,
            queue: x & Self::QU_MASK != 0,
            op_type: OpType::from(x & Self::OP_MASK),
        }
    }

    pub(crate) fn as_u8(self) -> u8 {
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

impl std::fmt::Display for ControlCode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "tcc: {:?} clear: {} queue: {} op_type: {:?}",
            self.tcc, self.clear, self.queue, self.op_type
        )
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
            0b1011_0100,
            ControlCode {
                tcc: TripCloseCode::Trip,
                clear: true,
                queue: true,
                op_type: OpType::LatchOff,
            },
        );

        test_control_code_round_trip(
            0b1001_0100,
            ControlCode {
                tcc: TripCloseCode::Trip,
                clear: false,
                queue: true,
                op_type: OpType::LatchOff,
            },
        );

        test_control_code_round_trip(
            0b1010_0100,
            ControlCode {
                tcc: TripCloseCode::Trip,
                clear: true,
                queue: false,
                op_type: OpType::LatchOff,
            },
        );

        test_control_code_round_trip(
            0b1100_0000,
            ControlCode {
                tcc: TripCloseCode::Reserved,
                clear: false,
                queue: false,
                op_type: OpType::Nul,
            },
        );
    }
}
