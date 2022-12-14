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
use crate::app::variations::*;

impl From<Group2Var2> for BinaryInput {
    fn from(v: Group2Var2) -> Self {
        let flags = Flags::new(v.flags);
        BinaryInput {
            value : flags.state(),
            flags,
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group2Var2> for BinaryInput {
    fn to_variation(&self) -> Group2Var2 {
        Group2Var2 {
            flags: self.get_wire_flags(),
            time: self.time.into(),
        }
    }
}

impl From<Group2Var1> for BinaryInput {
    fn from(v: Group2Var1) -> Self {
        let flags = Flags::new(v.flags);
        BinaryInput {
            value : flags.state(),
            flags,
            time : None,
        }
    }
}

impl ToVariation<Group2Var1> for BinaryInput {
    fn to_variation(&self) -> Group2Var1 {
        Group2Var1 {
            flags: self.get_wire_flags(),
        }
    }
}

impl From<Group1Var2> for BinaryInput {
    fn from(v: Group1Var2) -> Self {
        let flags = Flags::new(v.flags);
        BinaryInput {
            value : flags.state(),
            flags,
            time : None,
        }
    }
}

impl ToVariation<Group1Var2> for BinaryInput {
    fn to_variation(&self) -> Group1Var2 {
        Group1Var2 {
            flags: self.get_wire_flags(),
        }
    }
}


impl From<Group11Var2> for BinaryOutputStatus {
    fn from(v: Group11Var2) -> Self {
        let flags = Flags::new(v.flags);
        BinaryOutputStatus {
            value : flags.state(),
            flags,
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group11Var2> for BinaryOutputStatus {
    fn to_variation(&self) -> Group11Var2 {
        Group11Var2 {
            flags: self.get_wire_flags(),
            time: self.time.into(),
        }
    }
}

impl From<Group11Var1> for BinaryOutputStatus {
    fn from(v: Group11Var1) -> Self {
        let flags = Flags::new(v.flags);
        BinaryOutputStatus {
            value : flags.state(),
            flags,
            time : None,
        }
    }
}

impl ToVariation<Group11Var1> for BinaryOutputStatus {
    fn to_variation(&self) -> Group11Var1 {
        Group11Var1 {
            flags: self.get_wire_flags(),
        }
    }
}

impl From<Group10Var2> for BinaryOutputStatus {
    fn from(v: Group10Var2) -> Self {
        let flags = Flags::new(v.flags);
        BinaryOutputStatus {
            value : flags.state(),
            flags,
            time : None,
        }
    }
}

impl ToVariation<Group10Var2> for BinaryOutputStatus {
    fn to_variation(&self) -> Group10Var2 {
        Group10Var2 {
            flags: self.get_wire_flags(),
        }
    }
}


impl From<Group4Var2> for DoubleBitBinaryInput {
    fn from(v: Group4Var2) -> Self {
        let flags = Flags::new(v.flags);
        DoubleBitBinaryInput {
            value : flags.double_bit_state(),
            flags,
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group4Var2> for DoubleBitBinaryInput {
    fn to_variation(&self) -> Group4Var2 {
        Group4Var2 {
            flags: self.get_wire_flags(),
            time: self.time.into(),
        }
    }
}

impl From<Group4Var1> for DoubleBitBinaryInput {
    fn from(v: Group4Var1) -> Self {
        let flags = Flags::new(v.flags);
        DoubleBitBinaryInput {
            value : flags.double_bit_state(),
            flags,
            time : None,
        }
    }
}

impl ToVariation<Group4Var1> for DoubleBitBinaryInput {
    fn to_variation(&self) -> Group4Var1 {
        Group4Var1 {
            flags: self.get_wire_flags(),
        }
    }
}

impl From<Group3Var2> for DoubleBitBinaryInput {
    fn from(v: Group3Var2) -> Self {
        let flags = Flags::new(v.flags);
        DoubleBitBinaryInput {
            value : flags.double_bit_state(),
            flags,
            time : None,
        }
    }
}

impl ToVariation<Group3Var2> for DoubleBitBinaryInput {
    fn to_variation(&self) -> Group3Var2 {
        Group3Var2 {
            flags: self.get_wire_flags(),
        }
    }
}


impl From<Group22Var6> for Counter {
    fn from(v: Group22Var6) -> Self {
        Counter {
            value : v.value as u32,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group22Var6> for Counter {
    fn to_variation(&self) -> Group22Var6 {
        Group22Var6 {
            flags: self.flags.value,
            value: self.value as u16,
            time: self.time.into(),
        }
    }
}

impl From<Group22Var5> for Counter {
    fn from(v: Group22Var5) -> Self {
        Counter {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group22Var5> for Counter {
    fn to_variation(&self) -> Group22Var5 {
        Group22Var5 {
            flags: self.flags.value,
            value: self.value,
            time: self.time.into(),
        }
    }
}

impl From<Group22Var2> for Counter {
    fn from(v: Group22Var2) -> Self {
        Counter {
            value : v.value as u32,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group22Var2> for Counter {
    fn to_variation(&self) -> Group22Var2 {
        Group22Var2 {
            flags: self.flags.value,
            value: self.value as u16,
        }
    }
}

impl From<Group22Var1> for Counter {
    fn from(v: Group22Var1) -> Self {
        Counter {
            value : v.value,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group22Var1> for Counter {
    fn to_variation(&self) -> Group22Var1 {
        Group22Var1 {
            flags: self.flags.value,
            value: self.value,
        }
    }
}

impl From<Group20Var6> for Counter {
    fn from(v: Group20Var6) -> Self {
        Counter {
            value : v.value as u32,
            flags: Flags::ONLINE,
            time : None,
        }
    }
}

impl ToVariation<Group20Var6> for Counter {
    fn to_variation(&self) -> Group20Var6 {
        Group20Var6 {
            value: self.value as u16,
        }
    }
}

impl From<Group20Var5> for Counter {
    fn from(v: Group20Var5) -> Self {
        Counter {
            value : v.value,
            flags: Flags::ONLINE,
            time : None,
        }
    }
}

impl ToVariation<Group20Var5> for Counter {
    fn to_variation(&self) -> Group20Var5 {
        Group20Var5 {
            value: self.value,
        }
    }
}

impl From<Group20Var2> for Counter {
    fn from(v: Group20Var2) -> Self {
        Counter {
            value : v.value as u32,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group20Var2> for Counter {
    fn to_variation(&self) -> Group20Var2 {
        Group20Var2 {
            flags: self.flags.value,
            value: self.value as u16,
        }
    }
}

impl From<Group20Var1> for Counter {
    fn from(v: Group20Var1) -> Self {
        Counter {
            value : v.value,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group20Var1> for Counter {
    fn to_variation(&self) -> Group20Var1 {
        Group20Var1 {
            flags: self.flags.value,
            value: self.value,
        }
    }
}


impl From<Group23Var6> for FrozenCounter {
    fn from(v: Group23Var6) -> Self {
        FrozenCounter {
            value : v.value as u32,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group23Var6> for FrozenCounter {
    fn to_variation(&self) -> Group23Var6 {
        Group23Var6 {
            flags: self.flags.value,
            value: self.value as u16,
            time: self.time.into(),
        }
    }
}

impl From<Group23Var5> for FrozenCounter {
    fn from(v: Group23Var5) -> Self {
        FrozenCounter {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group23Var5> for FrozenCounter {
    fn to_variation(&self) -> Group23Var5 {
        Group23Var5 {
            flags: self.flags.value,
            value: self.value,
            time: self.time.into(),
        }
    }
}

impl From<Group23Var2> for FrozenCounter {
    fn from(v: Group23Var2) -> Self {
        FrozenCounter {
            value : v.value as u32,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group23Var2> for FrozenCounter {
    fn to_variation(&self) -> Group23Var2 {
        Group23Var2 {
            flags: self.flags.value,
            value: self.value as u16,
        }
    }
}

impl From<Group23Var1> for FrozenCounter {
    fn from(v: Group23Var1) -> Self {
        FrozenCounter {
            value : v.value,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group23Var1> for FrozenCounter {
    fn to_variation(&self) -> Group23Var1 {
        Group23Var1 {
            flags: self.flags.value,
            value: self.value,
        }
    }
}

impl From<Group21Var10> for FrozenCounter {
    fn from(v: Group21Var10) -> Self {
        FrozenCounter {
            value : v.value as u32,
            flags: Flags::ONLINE,
            time : None,
        }
    }
}

impl ToVariation<Group21Var10> for FrozenCounter {
    fn to_variation(&self) -> Group21Var10 {
        Group21Var10 {
            value: self.value as u16,
        }
    }
}

impl From<Group21Var9> for FrozenCounter {
    fn from(v: Group21Var9) -> Self {
        FrozenCounter {
            value : v.value,
            flags: Flags::ONLINE,
            time : None,
        }
    }
}

impl ToVariation<Group21Var9> for FrozenCounter {
    fn to_variation(&self) -> Group21Var9 {
        Group21Var9 {
            value: self.value,
        }
    }
}

impl From<Group21Var6> for FrozenCounter {
    fn from(v: Group21Var6) -> Self {
        FrozenCounter {
            value : v.value as u32,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group21Var6> for FrozenCounter {
    fn to_variation(&self) -> Group21Var6 {
        Group21Var6 {
            flags: self.flags.value,
            value: self.value as u16,
            time: self.time.into(),
        }
    }
}

impl From<Group21Var5> for FrozenCounter {
    fn from(v: Group21Var5) -> Self {
        FrozenCounter {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group21Var5> for FrozenCounter {
    fn to_variation(&self) -> Group21Var5 {
        Group21Var5 {
            flags: self.flags.value,
            value: self.value,
            time: self.time.into(),
        }
    }
}

impl From<Group21Var2> for FrozenCounter {
    fn from(v: Group21Var2) -> Self {
        FrozenCounter {
            value : v.value as u32,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group21Var2> for FrozenCounter {
    fn to_variation(&self) -> Group21Var2 {
        Group21Var2 {
            flags: self.flags.value,
            value: self.value as u16,
        }
    }
}

impl From<Group21Var1> for FrozenCounter {
    fn from(v: Group21Var1) -> Self {
        FrozenCounter {
            value : v.value,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group21Var1> for FrozenCounter {
    fn to_variation(&self) -> Group21Var1 {
        Group21Var1 {
            flags: self.flags.value,
            value: self.value,
        }
    }
}


impl From<Group32Var8> for AnalogInput {
    fn from(v: Group32Var8) -> Self {
        AnalogInput {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group32Var8> for AnalogInput {
    fn to_variation(&self) -> Group32Var8 {
        Group32Var8 {
            flags: self.flags.value,
            value: self.value,
            time: self.time.into(),
        }
    }
}

impl From<Group32Var7> for AnalogInput {
    fn from(v: Group32Var7) -> Self {
        AnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group32Var7> for AnalogInput {
    fn to_variation(&self) -> Group32Var7 {
        let (_wire_flags, _wire_value) = self.to_f32();
        Group32Var7 {
            flags: _wire_flags.value,
            value: _wire_value,
            time: self.time.into(),
        }
    }
}

impl From<Group32Var6> for AnalogInput {
    fn from(v: Group32Var6) -> Self {
        AnalogInput {
            value : v.value,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group32Var6> for AnalogInput {
    fn to_variation(&self) -> Group32Var6 {
        Group32Var6 {
            flags: self.flags.value,
            value: self.value,
        }
    }
}

impl From<Group32Var5> for AnalogInput {
    fn from(v: Group32Var5) -> Self {
        AnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group32Var5> for AnalogInput {
    fn to_variation(&self) -> Group32Var5 {
        let (_wire_flags, _wire_value) = self.to_f32();
        Group32Var5 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}

impl From<Group32Var4> for AnalogInput {
    fn from(v: Group32Var4) -> Self {
        AnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group32Var4> for AnalogInput {
    fn to_variation(&self) -> Group32Var4 {
        let (_wire_flags, _wire_value) = self.to_i16();
        Group32Var4 {
            flags: _wire_flags.value,
            value: _wire_value,
            time: self.time.into(),
        }
    }
}

impl From<Group32Var3> for AnalogInput {
    fn from(v: Group32Var3) -> Self {
        AnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group32Var3> for AnalogInput {
    fn to_variation(&self) -> Group32Var3 {
        let (_wire_flags, _wire_value) = self.to_i32();
        Group32Var3 {
            flags: _wire_flags.value,
            value: _wire_value,
            time: self.time.into(),
        }
    }
}

impl From<Group32Var2> for AnalogInput {
    fn from(v: Group32Var2) -> Self {
        AnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group32Var2> for AnalogInput {
    fn to_variation(&self) -> Group32Var2 {
        let (_wire_flags, _wire_value) = self.to_i16();
        Group32Var2 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}

impl From<Group32Var1> for AnalogInput {
    fn from(v: Group32Var1) -> Self {
        AnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group32Var1> for AnalogInput {
    fn to_variation(&self) -> Group32Var1 {
        let (_wire_flags, _wire_value) = self.to_i32();
        Group32Var1 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}

impl From<Group30Var6> for AnalogInput {
    fn from(v: Group30Var6) -> Self {
        AnalogInput {
            value : v.value,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group30Var6> for AnalogInput {
    fn to_variation(&self) -> Group30Var6 {
        Group30Var6 {
            flags: self.flags.value,
            value: self.value,
        }
    }
}

impl From<Group30Var5> for AnalogInput {
    fn from(v: Group30Var5) -> Self {
        AnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group30Var5> for AnalogInput {
    fn to_variation(&self) -> Group30Var5 {
        let (_wire_flags, _wire_value) = self.to_f32();
        Group30Var5 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}

impl From<Group30Var4> for AnalogInput {
    fn from(v: Group30Var4) -> Self {
        AnalogInput {
            value : v.value as f64,
            flags: Flags::ONLINE,
            time : None,
        }
    }
}

impl ToVariation<Group30Var4> for AnalogInput {
    fn to_variation(&self) -> Group30Var4 {
        let (_wire_flags, _wire_value) = self.to_i16();
        Group30Var4 {
            value: _wire_value,
        }
    }
}

impl From<Group30Var3> for AnalogInput {
    fn from(v: Group30Var3) -> Self {
        AnalogInput {
            value : v.value as f64,
            flags: Flags::ONLINE,
            time : None,
        }
    }
}

impl ToVariation<Group30Var3> for AnalogInput {
    fn to_variation(&self) -> Group30Var3 {
        let (_wire_flags, _wire_value) = self.to_i32();
        Group30Var3 {
            value: _wire_value,
        }
    }
}

impl From<Group30Var2> for AnalogInput {
    fn from(v: Group30Var2) -> Self {
        AnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group30Var2> for AnalogInput {
    fn to_variation(&self) -> Group30Var2 {
        let (_wire_flags, _wire_value) = self.to_i16();
        Group30Var2 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}

impl From<Group30Var1> for AnalogInput {
    fn from(v: Group30Var1) -> Self {
        AnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group30Var1> for AnalogInput {
    fn to_variation(&self) -> Group30Var1 {
        let (_wire_flags, _wire_value) = self.to_i32();
        Group30Var1 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}


impl From<Group42Var8> for AnalogOutputStatus {
    fn from(v: Group42Var8) -> Self {
        AnalogOutputStatus {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group42Var8> for AnalogOutputStatus {
    fn to_variation(&self) -> Group42Var8 {
        Group42Var8 {
            flags: self.flags.value,
            value: self.value,
            time: self.time.into(),
        }
    }
}

impl From<Group42Var7> for AnalogOutputStatus {
    fn from(v: Group42Var7) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group42Var7> for AnalogOutputStatus {
    fn to_variation(&self) -> Group42Var7 {
        let (_wire_flags, _wire_value) = self.to_f32();
        Group42Var7 {
            flags: _wire_flags.value,
            value: _wire_value,
            time: self.time.into(),
        }
    }
}

impl From<Group42Var6> for AnalogOutputStatus {
    fn from(v: Group42Var6) -> Self {
        AnalogOutputStatus {
            value : v.value,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group42Var6> for AnalogOutputStatus {
    fn to_variation(&self) -> Group42Var6 {
        Group42Var6 {
            flags: self.flags.value,
            value: self.value,
        }
    }
}

impl From<Group42Var5> for AnalogOutputStatus {
    fn from(v: Group42Var5) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group42Var5> for AnalogOutputStatus {
    fn to_variation(&self) -> Group42Var5 {
        let (_wire_flags, _wire_value) = self.to_f32();
        Group42Var5 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}

impl From<Group42Var4> for AnalogOutputStatus {
    fn from(v: Group42Var4) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group42Var4> for AnalogOutputStatus {
    fn to_variation(&self) -> Group42Var4 {
        let (_wire_flags, _wire_value) = self.to_i16();
        Group42Var4 {
            flags: _wire_flags.value,
            value: _wire_value,
            time: self.time.into(),
        }
    }
}

impl From<Group42Var3> for AnalogOutputStatus {
    fn from(v: Group42Var3) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group42Var3> for AnalogOutputStatus {
    fn to_variation(&self) -> Group42Var3 {
        let (_wire_flags, _wire_value) = self.to_i32();
        Group42Var3 {
            flags: _wire_flags.value,
            value: _wire_value,
            time: self.time.into(),
        }
    }
}

impl From<Group42Var2> for AnalogOutputStatus {
    fn from(v: Group42Var2) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group42Var2> for AnalogOutputStatus {
    fn to_variation(&self) -> Group42Var2 {
        let (_wire_flags, _wire_value) = self.to_i16();
        Group42Var2 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}

impl From<Group42Var1> for AnalogOutputStatus {
    fn from(v: Group42Var1) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group42Var1> for AnalogOutputStatus {
    fn to_variation(&self) -> Group42Var1 {
        let (_wire_flags, _wire_value) = self.to_i32();
        Group42Var1 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}

impl From<Group40Var4> for AnalogOutputStatus {
    fn from(v: Group40Var4) -> Self {
        AnalogOutputStatus {
            value : v.value,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group40Var4> for AnalogOutputStatus {
    fn to_variation(&self) -> Group40Var4 {
        Group40Var4 {
            flags: self.flags.value,
            value: self.value,
        }
    }
}

impl From<Group40Var3> for AnalogOutputStatus {
    fn from(v: Group40Var3) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group40Var3> for AnalogOutputStatus {
    fn to_variation(&self) -> Group40Var3 {
        let (_wire_flags, _wire_value) = self.to_f32();
        Group40Var3 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}

impl From<Group40Var2> for AnalogOutputStatus {
    fn from(v: Group40Var2) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group40Var2> for AnalogOutputStatus {
    fn to_variation(&self) -> Group40Var2 {
        let (_wire_flags, _wire_value) = self.to_i16();
        Group40Var2 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}

impl From<Group40Var1> for AnalogOutputStatus {
    fn from(v: Group40Var1) -> Self {
        AnalogOutputStatus {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group40Var1> for AnalogOutputStatus {
    fn to_variation(&self) -> Group40Var1 {
        let (_wire_flags, _wire_value) = self.to_i32();
        Group40Var1 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}


impl From<Group33Var8> for FrozenAnalogInput {
    fn from(v: Group33Var8) -> Self {
        FrozenAnalogInput {
            value : v.value,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group33Var8> for FrozenAnalogInput {
    fn to_variation(&self) -> Group33Var8 {
        Group33Var8 {
            flags: self.flags.value,
            value: self.value,
            time: self.time.into(),
        }
    }
}

impl From<Group33Var7> for FrozenAnalogInput {
    fn from(v: Group33Var7) -> Self {
        FrozenAnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group33Var7> for FrozenAnalogInput {
    fn to_variation(&self) -> Group33Var7 {
        let (_wire_flags, _wire_value) = self.to_f32();
        Group33Var7 {
            flags: _wire_flags.value,
            value: _wire_value,
            time: self.time.into(),
        }
    }
}

impl From<Group33Var6> for FrozenAnalogInput {
    fn from(v: Group33Var6) -> Self {
        FrozenAnalogInput {
            value : v.value,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group33Var6> for FrozenAnalogInput {
    fn to_variation(&self) -> Group33Var6 {
        Group33Var6 {
            flags: self.flags.value,
            value: self.value,
        }
    }
}

impl From<Group33Var5> for FrozenAnalogInput {
    fn from(v: Group33Var5) -> Self {
        FrozenAnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group33Var5> for FrozenAnalogInput {
    fn to_variation(&self) -> Group33Var5 {
        let (_wire_flags, _wire_value) = self.to_f32();
        Group33Var5 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}

impl From<Group33Var4> for FrozenAnalogInput {
    fn from(v: Group33Var4) -> Self {
        FrozenAnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group33Var4> for FrozenAnalogInput {
    fn to_variation(&self) -> Group33Var4 {
        let (_wire_flags, _wire_value) = self.to_i16();
        Group33Var4 {
            flags: _wire_flags.value,
            value: _wire_value,
            time: self.time.into(),
        }
    }
}

impl From<Group33Var3> for FrozenAnalogInput {
    fn from(v: Group33Var3) -> Self {
        FrozenAnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group33Var3> for FrozenAnalogInput {
    fn to_variation(&self) -> Group33Var3 {
        let (_wire_flags, _wire_value) = self.to_i32();
        Group33Var3 {
            flags: _wire_flags.value,
            value: _wire_value,
            time: self.time.into(),
        }
    }
}

impl From<Group33Var2> for FrozenAnalogInput {
    fn from(v: Group33Var2) -> Self {
        FrozenAnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group33Var2> for FrozenAnalogInput {
    fn to_variation(&self) -> Group33Var2 {
        let (_wire_flags, _wire_value) = self.to_i16();
        Group33Var2 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}

impl From<Group33Var1> for FrozenAnalogInput {
    fn from(v: Group33Var1) -> Self {
        FrozenAnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group33Var1> for FrozenAnalogInput {
    fn to_variation(&self) -> Group33Var1 {
        let (_wire_flags, _wire_value) = self.to_i32();
        Group33Var1 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}

impl From<Group31Var8> for FrozenAnalogInput {
    fn from(v: Group31Var8) -> Self {
        FrozenAnalogInput {
            value : v.value,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group31Var8> for FrozenAnalogInput {
    fn to_variation(&self) -> Group31Var8 {
        Group31Var8 {
            flags: self.flags.value,
            value: self.value,
        }
    }
}

impl From<Group31Var7> for FrozenAnalogInput {
    fn from(v: Group31Var7) -> Self {
        FrozenAnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group31Var7> for FrozenAnalogInput {
    fn to_variation(&self) -> Group31Var7 {
        let (_wire_flags, _wire_value) = self.to_f32();
        Group31Var7 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}

impl From<Group31Var6> for FrozenAnalogInput {
    fn from(v: Group31Var6) -> Self {
        FrozenAnalogInput {
            value : v.value as f64,
            flags: Flags::ONLINE,
            time : None,
        }
    }
}

impl ToVariation<Group31Var6> for FrozenAnalogInput {
    fn to_variation(&self) -> Group31Var6 {
        let (_wire_flags, _wire_value) = self.to_i16();
        Group31Var6 {
            value: _wire_value,
        }
    }
}

impl From<Group31Var5> for FrozenAnalogInput {
    fn from(v: Group31Var5) -> Self {
        FrozenAnalogInput {
            value : v.value as f64,
            flags: Flags::ONLINE,
            time : None,
        }
    }
}

impl ToVariation<Group31Var5> for FrozenAnalogInput {
    fn to_variation(&self) -> Group31Var5 {
        let (_wire_flags, _wire_value) = self.to_i32();
        Group31Var5 {
            value: _wire_value,
        }
    }
}

impl From<Group31Var4> for FrozenAnalogInput {
    fn from(v: Group31Var4) -> Self {
        FrozenAnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group31Var4> for FrozenAnalogInput {
    fn to_variation(&self) -> Group31Var4 {
        let (_wire_flags, _wire_value) = self.to_i16();
        Group31Var4 {
            flags: _wire_flags.value,
            value: _wire_value,
            time: self.time.into(),
        }
    }
}

impl From<Group31Var3> for FrozenAnalogInput {
    fn from(v: Group31Var3) -> Self {
        FrozenAnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : Some(Time::Synchronized(v.time)),
        }
    }
}

impl ToVariation<Group31Var3> for FrozenAnalogInput {
    fn to_variation(&self) -> Group31Var3 {
        let (_wire_flags, _wire_value) = self.to_i32();
        Group31Var3 {
            flags: _wire_flags.value,
            value: _wire_value,
            time: self.time.into(),
        }
    }
}

impl From<Group31Var2> for FrozenAnalogInput {
    fn from(v: Group31Var2) -> Self {
        FrozenAnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group31Var2> for FrozenAnalogInput {
    fn to_variation(&self) -> Group31Var2 {
        let (_wire_flags, _wire_value) = self.to_i16();
        Group31Var2 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}

impl From<Group31Var1> for FrozenAnalogInput {
    fn from(v: Group31Var1) -> Self {
        FrozenAnalogInput {
            value : v.value as f64,
            flags: Flags::new(v.flags),
            time : None,
        }
    }
}

impl ToVariation<Group31Var1> for FrozenAnalogInput {
    fn to_variation(&self) -> Group31Var1 {
        let (_wire_flags, _wire_value) = self.to_i32();
        Group31Var1 {
            flags: _wire_flags.value,
            value: _wire_value,
        }
    }
}

