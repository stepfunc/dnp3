use crate::app::measurement::{Binary, BinaryOutputStatus, DoubleBitBinary, Time};
use crate::app::variations::{Group2Var3, Group4Var3};
use crate::app::DoubleBit;
use crate::app::Flags;

impl Group2Var3 {
    pub(crate) fn to_measurement(self, cto: Option<Time>) -> Binary {
        let flags = Flags::new(self.flags);
        Binary {
            value: flags.state(),
            flags,
            time: cto.and_then(|x| x.checked_add(self.time)),
        }
    }
}

impl Group4Var3 {
    pub(crate) fn to_measurement(self, cto: Option<Time>) -> DoubleBitBinary {
        let flags = Flags::new(self.flags);
        DoubleBitBinary {
            value: flags.double_bit_state(),
            flags,
            time: cto.and_then(|x| x.checked_add(self.time)),
        }
    }
}

impl From<bool> for Binary {
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

impl From<DoubleBit> for DoubleBitBinary {
    fn from(x: DoubleBit) -> Self {
        Self {
            value: x,
            flags: Flags::ONLINE,
            time: None,
        }
    }
}
