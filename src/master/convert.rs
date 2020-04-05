use crate::app::flags::Flags;
use crate::app::measurement::{Binary, BinaryOutputStatus, DoubleBitBinary, Time};
use crate::app::types::DoubleBit;

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
