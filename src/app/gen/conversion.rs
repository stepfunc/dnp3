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

use crate::app::measurement::*;
use crate::app::flags::*;
use crate::app::gen::variations::fixed::*;

impl std::convert::From<Group2Var2> for Binary {
    fn from(v: Group2Var2) -> Self {
        let flags = Flags::new(v.flags);
        Binary {
            value : flags.state(),
            flags,
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group2Var1> for Binary {
    fn from(v: Group2Var1) -> Self {
        let flags = Flags::new(v.flags);
        Binary {
            value : flags.state(),
            flags,
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group1Var2> for Binary {
    fn from(v: Group1Var2) -> Self {
        let flags = Flags::new(v.flags);
        Binary {
            value : flags.state(),
            flags,
            time : Time::Invalid,
        }
    }
}


impl std::convert::From<Group13Var2> for BinaryOutputStatus {
    fn from(v: Group13Var2) -> Self {
        let flags = Flags::new(v.flags);
        BinaryOutputStatus {
            value : flags.state(),
            flags,
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group13Var1> for BinaryOutputStatus {
    fn from(v: Group13Var1) -> Self {
        let flags = Flags::new(v.flags);
        BinaryOutputStatus {
            value : flags.state(),
            flags,
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group11Var2> for BinaryOutputStatus {
    fn from(v: Group11Var2) -> Self {
        let flags = Flags::new(v.flags);
        BinaryOutputStatus {
            value : flags.state(),
            flags,
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group11Var1> for BinaryOutputStatus {
    fn from(v: Group11Var1) -> Self {
        let flags = Flags::new(v.flags);
        BinaryOutputStatus {
            value : flags.state(),
            flags,
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group10Var2> for BinaryOutputStatus {
    fn from(v: Group10Var2) -> Self {
        let flags = Flags::new(v.flags);
        BinaryOutputStatus {
            value : flags.state(),
            flags,
            time : Time::Invalid,
        }
    }
}


impl std::convert::From<Group4Var2> for DoubleBitBinary {
    fn from(v: Group4Var2) -> Self {
        let flags = Flags::new(v.flags);
        DoubleBitBinary {
            value : flags.double_bit_state(),
            flags,
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group4Var1> for DoubleBitBinary {
    fn from(v: Group4Var1) -> Self {
        let flags = Flags::new(v.flags);
        DoubleBitBinary {
            value : flags.double_bit_state(),
            flags,
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group3Var2> for DoubleBitBinary {
    fn from(v: Group3Var2) -> Self {
        let flags = Flags::new(v.flags);
        DoubleBitBinary {
            value : flags.double_bit_state(),
            flags,
            time : Time::Invalid,
        }
    }
}


impl std::convert::From<Group22Var6> for Counter {
    fn from(v: Group22Var6) -> Self {
        Counter {
            value : v.value as u32,
            flags: Flags::new(v.flags),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group22Var5> for Counter {
    fn from(v: Group22Var5) -> Self {
        Counter {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group22Var2> for Counter {
    fn from(v: Group22Var2) -> Self {
        Counter {
            value : v.value as u32,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group22Var1> for Counter {
    fn from(v: Group22Var1) -> Self {
        Counter {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group20Var6> for Counter {
    fn from(v: Group20Var6) -> Self {
        Counter {
            value : v.value as u32,
            flags: Flags::new(masks::ONLINE),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group20Var5> for Counter {
    fn from(v: Group20Var5) -> Self {
        Counter {
            value : v.value,
            flags: Flags::new(masks::ONLINE),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group20Var2> for Counter {
    fn from(v: Group20Var2) -> Self {
        Counter {
            value : v.value as u32,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group20Var1> for Counter {
    fn from(v: Group20Var1) -> Self {
        Counter {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}


impl std::convert::From<Group23Var6> for FrozenCounter {
    fn from(v: Group23Var6) -> Self {
        FrozenCounter {
            value : v.value as u32,
            flags: Flags::new(v.flags),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group23Var5> for FrozenCounter {
    fn from(v: Group23Var5) -> Self {
        FrozenCounter {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group23Var2> for FrozenCounter {
    fn from(v: Group23Var2) -> Self {
        FrozenCounter {
            value : v.value as u32,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group23Var1> for FrozenCounter {
    fn from(v: Group23Var1) -> Self {
        FrozenCounter {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group21Var10> for FrozenCounter {
    fn from(v: Group21Var10) -> Self {
        FrozenCounter {
            value : v.value as u32,
            flags: Flags::new(masks::ONLINE),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group21Var9> for FrozenCounter {
    fn from(v: Group21Var9) -> Self {
        FrozenCounter {
            value : v.value,
            flags: Flags::new(masks::ONLINE),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group21Var6> for FrozenCounter {
    fn from(v: Group21Var6) -> Self {
        FrozenCounter {
            value : v.value as u32,
            flags: Flags::new(v.flags),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group21Var5> for FrozenCounter {
    fn from(v: Group21Var5) -> Self {
        FrozenCounter {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group21Var2> for FrozenCounter {
    fn from(v: Group21Var2) -> Self {
        FrozenCounter {
            value : v.value as u32,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group21Var1> for FrozenCounter {
    fn from(v: Group21Var1) -> Self {
        FrozenCounter {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}


impl std::convert::From<Group32Var8> for Analog {
    fn from(v: Group32Var8) -> Self {
        Analog {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group32Var7> for Analog {
    fn from(v: Group32Var7) -> Self {
        Analog {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group32Var6> for Analog {
    fn from(v: Group32Var6) -> Self {
        Analog {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group32Var5> for Analog {
    fn from(v: Group32Var5) -> Self {
        Analog {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group32Var4> for Analog {
    fn from(v: Group32Var4) -> Self {
        Analog {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group32Var3> for Analog {
    fn from(v: Group32Var3) -> Self {
        Analog {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group32Var2> for Analog {
    fn from(v: Group32Var2) -> Self {
        Analog {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group32Var1> for Analog {
    fn from(v: Group32Var1) -> Self {
        Analog {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group30Var6> for Analog {
    fn from(v: Group30Var6) -> Self {
        Analog {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group30Var5> for Analog {
    fn from(v: Group30Var5) -> Self {
        Analog {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group30Var4> for Analog {
    fn from(v: Group30Var4) -> Self {
        Analog {
            value : v.value as f64,
            flags: Flags::new(masks::ONLINE),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group30Var3> for Analog {
    fn from(v: Group30Var3) -> Self {
        Analog {
            value : v.value as f64,
            flags: Flags::new(masks::ONLINE),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group30Var2> for Analog {
    fn from(v: Group30Var2) -> Self {
        Analog {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group30Var1> for Analog {
    fn from(v: Group30Var1) -> Self {
        Analog {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}


impl std::convert::From<Group43Var8> for AnalogOutputStatus {
    fn from(v: Group43Var8) -> Self {
        AnalogOutputStatus {
            value : v.value,
            flags: Flags::new(masks::ONLINE),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group43Var7> for AnalogOutputStatus {
    fn from(v: Group43Var7) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(masks::ONLINE),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group43Var6> for AnalogOutputStatus {
    fn from(v: Group43Var6) -> Self {
        AnalogOutputStatus {
            value : v.value,
            flags: Flags::new(masks::ONLINE),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group43Var5> for AnalogOutputStatus {
    fn from(v: Group43Var5) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(masks::ONLINE),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group43Var4> for AnalogOutputStatus {
    fn from(v: Group43Var4) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(masks::ONLINE),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group43Var3> for AnalogOutputStatus {
    fn from(v: Group43Var3) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(masks::ONLINE),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group43Var2> for AnalogOutputStatus {
    fn from(v: Group43Var2) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(masks::ONLINE),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group43Var1> for AnalogOutputStatus {
    fn from(v: Group43Var1) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(masks::ONLINE),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group42Var8> for AnalogOutputStatus {
    fn from(v: Group42Var8) -> Self {
        AnalogOutputStatus {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group42Var7> for AnalogOutputStatus {
    fn from(v: Group42Var7) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group42Var6> for AnalogOutputStatus {
    fn from(v: Group42Var6) -> Self {
        AnalogOutputStatus {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group42Var5> for AnalogOutputStatus {
    fn from(v: Group42Var5) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group42Var4> for AnalogOutputStatus {
    fn from(v: Group42Var4) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group42Var3> for AnalogOutputStatus {
    fn from(v: Group42Var3) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group42Var2> for AnalogOutputStatus {
    fn from(v: Group42Var2) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group42Var1> for AnalogOutputStatus {
    fn from(v: Group42Var1) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group40Var4> for AnalogOutputStatus {
    fn from(v: Group40Var4) -> Self {
        AnalogOutputStatus {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group40Var3> for AnalogOutputStatus {
    fn from(v: Group40Var3) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group40Var2> for AnalogOutputStatus {
    fn from(v: Group40Var2) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

impl std::convert::From<Group40Var1> for AnalogOutputStatus {
    fn from(v: Group40Var1) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Time::Invalid,
        }
    }
}

