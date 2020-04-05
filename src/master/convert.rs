use crate::app::flags::Flags;
use crate::app::gen::variations::fixed::{Group2Var3, Group4Var3};
use crate::app::measurement::{Binary, BinaryOutputStatus, DoubleBitBinary, Time};
use crate::app::types::DoubleBit;

impl Group2Var3 {
    pub(crate) fn to_measurement(&self, cto: Time) -> Binary {
        let flags = Flags::new(self.flags);
        Binary {
            value: flags.state(),
            flags,
            time: cto.checked_add(self.time),
        }
    }
}

impl Group4Var3 {
    pub(crate) fn to_measurement(&self, cto: Time) -> DoubleBitBinary {
        let flags = Flags::new(self.flags);
        DoubleBitBinary {
            value: flags.double_bit_state(),
            flags,
            time: cto.checked_add(self.time),
        }
    }
}

impl std::convert::From<bool> for Binary {
    fn from(x: bool) -> Self {
        Self {
            value: x,
            flags: Flags::ONLINE,
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<bool> for BinaryOutputStatus {
    fn from(x: bool) -> Self {
        Self {
            value: x,
            flags: Flags::ONLINE,
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<DoubleBit> for DoubleBitBinary {
    fn from(x: DoubleBit) -> Self {
        Self {
            value: x,
            flags: Flags::ONLINE,
            time: Time::Invalid,
        }
    }
}
