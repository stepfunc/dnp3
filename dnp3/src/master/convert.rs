use crate::app::measurement::*;
use crate::app::variations::{Group2Var3, Group4Var3};

impl Group2Var3 {
    pub(crate) fn to_measurement(self, cto: Option<Time>) -> BinaryInput {
        let flags = Flags::new(self.flags);
        BinaryInput {
            value: flags.state(),
            flags,
            time: cto.and_then(|x| x.checked_add(self.time)),
        }
    }
}

impl Group4Var3 {
    pub(crate) fn to_measurement(self, cto: Option<Time>) -> DoubleBitBinaryInput {
        let flags = Flags::new(self.flags);
        DoubleBitBinaryInput {
            value: flags.double_bit_state(),
            flags,
            time: cto.and_then(|x| x.checked_add(self.time)),
        }
    }
}

impl From<bool> for BinaryInput {
    fn from(x: bool) -> Self {
        Self {
            value: x,
            flags: Flags::ONLINE,
            time: None,
        }
    }
}

impl From<bool> for BinaryOutputStatus {
    fn from(x: bool) -> Self {
        Self {
            value: x,
            flags: Flags::ONLINE,
            time: None,
        }
    }
}

impl From<DoubleBit> for DoubleBitBinaryInput {
    fn from(x: DoubleBit) -> Self {
        Self {
            value: x,
            flags: Flags::ONLINE,
            time: None,
        }
    }
}
