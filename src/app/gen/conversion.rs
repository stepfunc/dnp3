//  _   _         ______    _ _ _   _             _ _ _
// | \ | |       |  ____|  | (_) | (_)           | | | |
// |  \| | ___   | |__   __| |_| |_ _ _ __   __ _| | | |
// | . ` |/ _ \  |  __| / _` | | __| | '_ \ / _` | | | |
// | |\  | (_) | | |___| (_| | | |_| | | | | (_| |_|_|_|
// |_| \_|\___/  |______\__,_|_|\__|_|_| |_|\__, (_|_|_)
//                                           __/ |
//                                          |___/
//
// This file is auto-generated. Do not edit manually
//

use crate::app::flags::*;
use crate::app::gen::variations::fixed::*;
use crate::app::meas::*;

impl std::convert::From<Group2Var2> for Binary {
    fn from(v: Group2Var2) -> Self {
        let flags = Flags::new(v.flags);
        Binary {
            value: flags.state(),
            flags,
            time: Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group2Var1> for Binary {
    fn from(v: Group2Var1) -> Self {
        let flags = Flags::new(v.flags);
        Binary {
            value: flags.state(),
            flags,
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<Group1Var2> for Binary {
    fn from(v: Group1Var2) -> Self {
        let flags = Flags::new(v.flags);
        Binary {
            value: flags.state(),
            flags,
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<Group4Var2> for DoubleBitBinary {
    fn from(v: Group4Var2) -> Self {
        let flags = Flags::new(v.flags);
        DoubleBitBinary {
            value: flags.double_bit_state(),
            flags,
            time: Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group4Var1> for DoubleBitBinary {
    fn from(v: Group4Var1) -> Self {
        let flags = Flags::new(v.flags);
        DoubleBitBinary {
            value: flags.double_bit_state(),
            flags,
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<Group3Var2> for DoubleBitBinary {
    fn from(v: Group3Var2) -> Self {
        let flags = Flags::new(v.flags);
        DoubleBitBinary {
            value: flags.double_bit_state(),
            flags,
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<Group22Var6> for Counter {
    fn from(v: Group22Var6) -> Self {
        Counter {
            value: v.value as u32,
            flags: Flags::new(v.flags),
            time: Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group22Var5> for Counter {
    fn from(v: Group22Var5) -> Self {
        Counter {
            value: v.value,
            flags: Flags::new(v.flags),
            time: Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group22Var2> for Counter {
    fn from(v: Group22Var2) -> Self {
        Counter {
            value: v.value as u32,
            flags: Flags::new(v.flags),
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<Group22Var1> for Counter {
    fn from(v: Group22Var1) -> Self {
        Counter {
            value: v.value,
            flags: Flags::new(v.flags),
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<Group20Var6> for Counter {
    fn from(v: Group20Var6) -> Self {
        Counter {
            value: v.value as u32,
            flags: Flags::new(masks::ONLINE),
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<Group20Var5> for Counter {
    fn from(v: Group20Var5) -> Self {
        Counter {
            value: v.value,
            flags: Flags::new(masks::ONLINE),
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<Group20Var2> for Counter {
    fn from(v: Group20Var2) -> Self {
        Counter {
            value: v.value as u32,
            flags: Flags::new(v.flags),
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<Group20Var1> for Counter {
    fn from(v: Group20Var1) -> Self {
        Counter {
            value: v.value,
            flags: Flags::new(v.flags),
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<Group23Var6> for FrozenCounter {
    fn from(v: Group23Var6) -> Self {
        FrozenCounter {
            value: v.value as u32,
            flags: Flags::new(v.flags),
            time: Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group23Var5> for FrozenCounter {
    fn from(v: Group23Var5) -> Self {
        FrozenCounter {
            value: v.value,
            flags: Flags::new(v.flags),
            time: Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group23Var2> for FrozenCounter {
    fn from(v: Group23Var2) -> Self {
        FrozenCounter {
            value: v.value as u32,
            flags: Flags::new(v.flags),
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<Group23Var1> for FrozenCounter {
    fn from(v: Group23Var1) -> Self {
        FrozenCounter {
            value: v.value,
            flags: Flags::new(v.flags),
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<Group21Var10> for FrozenCounter {
    fn from(v: Group21Var10) -> Self {
        FrozenCounter {
            value: v.value as u32,
            flags: Flags::new(masks::ONLINE),
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<Group21Var9> for FrozenCounter {
    fn from(v: Group21Var9) -> Self {
        FrozenCounter {
            value: v.value,
            flags: Flags::new(masks::ONLINE),
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<Group21Var6> for FrozenCounter {
    fn from(v: Group21Var6) -> Self {
        FrozenCounter {
            value: v.value as u32,
            flags: Flags::new(v.flags),
            time: Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group21Var5> for FrozenCounter {
    fn from(v: Group21Var5) -> Self {
        FrozenCounter {
            value: v.value,
            flags: Flags::new(v.flags),
            time: Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group21Var2> for FrozenCounter {
    fn from(v: Group21Var2) -> Self {
        FrozenCounter {
            value: v.value as u32,
            flags: Flags::new(v.flags),
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<Group21Var1> for FrozenCounter {
    fn from(v: Group21Var1) -> Self {
        FrozenCounter {
            value: v.value,
            flags: Flags::new(v.flags),
            time: Time::Invalid,
        }
    }
}
