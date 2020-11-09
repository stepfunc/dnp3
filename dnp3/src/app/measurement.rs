use crate::app::flags::Flags;
use crate::app::types::{DoubleBit, Timestamp};
use crate::util::bit::bits::*;
use crate::util::bit::BitMask;

pub(crate) trait ToVariation<V> {
    fn to_variation(&self) -> V;
}

pub(crate) trait WireFlags {
    fn get_wire_flags(&self) -> u8;
}

/// A DNP3 time value that may be Invalid, Synchronized, or NotSynchronized
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Time {
    /// The timestamp is UTC synchronized at the remote device
    Synchronized(Timestamp),
    /// The device indicates the timestamp may be not be synchronized
    NotSynchronized(Timestamp),
}

impl Time {
    pub fn is_synchronized(&self) -> bool {
        std::matches!(self, Self::Synchronized(_))
    }

    pub fn synchronized(ts: u64) -> Time {
        Self::Synchronized(Timestamp::new(ts))
    }

    pub fn not_synchronized(ts: u64) -> Time {
        Self::NotSynchronized(Timestamp::new(ts))
    }
}

impl From<Option<Time>> for Time {
    fn from(x: Option<Time>) -> Self {
        x.unwrap_or_else(|| Time::NotSynchronized(Timestamp::new(0)))
    }
}

impl From<Option<Time>> for Timestamp {
    fn from(x: Option<Time>) -> Self {
        Time::from(x).timestamp()
    }
}

impl Time {
    pub(crate) fn checked_add(self, x: u16) -> Option<Self> {
        match self {
            Time::Synchronized(ts) => match ts.checked_add(x) {
                Some(x) => Some(Time::Synchronized(x)),
                None => None,
            },
            Time::NotSynchronized(ts) => match ts.checked_add(x) {
                Some(x) => Some(Time::NotSynchronized(x)),
                None => None,
            },
        }
    }

    pub fn timestamp(&self) -> Timestamp {
        match self {
            Time::Synchronized(ts) => *ts,
            Time::NotSynchronized(ts) => *ts,
        }
    }
}

/// Measurement type corresponding to groups 1 and 2
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Binary {
    /// value of the type
    pub value: bool,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Option<Time>,
}

impl Binary {
    pub fn new(value: bool, flags: Flags, time: Time) -> Self {
        Self {
            value,
            flags,
            time: Some(time),
        }
    }
}

/// Measurement type corresponding to groups 3 and 4
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct DoubleBitBinary {
    /// value of the type
    pub value: DoubleBit,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Option<Time>,
}

impl DoubleBitBinary {
    pub fn new(value: DoubleBit, flags: Flags, time: Time) -> Self {
        Self {
            value,
            flags,
            time: Some(time),
        }
    }
}

/// Measurement type corresponding to groups 10 and 11
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct BinaryOutputStatus {
    /// value of the type
    pub value: bool,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Option<Time>,
}

impl BinaryOutputStatus {
    pub fn new(value: bool, flags: Flags, time: Time) -> Self {
        Self {
            value,
            flags,
            time: Some(time),
        }
    }
}

/// Measurement type corresponding to groups 20 and 22
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Counter {
    /// value of the type
    pub value: u32,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Option<Time>,
}

impl Counter {
    pub fn new(value: u32, flags: Flags, time: Time) -> Self {
        Self {
            value,
            flags,
            time: Some(time),
        }
    }
}

/// Measurement type corresponding to groups 21 and 23
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct FrozenCounter {
    /// value of the type
    pub value: u32,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Option<Time>,
}

impl FrozenCounter {
    pub fn new(value: u32, flags: Flags, time: Time) -> Self {
        Self {
            value,
            flags,
            time: Some(time),
        }
    }
}

/// Measurement type corresponding to groups 30 and 32
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Analog {
    /// value of the type
    pub value: f64,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Option<Time>,
}

impl Analog {
    pub fn new(value: f64, flags: Flags, time: Time) -> Self {
        Self {
            value,
            flags,
            time: Some(time),
        }
    }
}

pub(crate) trait AnalogConversions {
    const OVER_RANGE: BitMask = BIT_5;

    fn get_value(&self) -> f64;
    fn get_flags(&self) -> Flags;

    fn to_i16(&self) -> (Flags, i16) {
        if self.get_value() < i16::MIN.into() {
            return (self.get_flags().with_bits_set(Self::OVER_RANGE), i16::MIN);
        }

        if self.get_value() > i16::MAX.into() {
            return (self.get_flags().with_bits_set(Self::OVER_RANGE), i16::MAX);
        }

        (self.get_flags(), self.get_value() as i16)
    }

    fn to_i32(&self) -> (Flags, i32) {
        if self.get_value() < i32::MIN.into() {
            return (self.get_flags().with_bits_set(Self::OVER_RANGE), i32::MIN);
        }

        if self.get_value() > i32::MAX.into() {
            return (self.get_flags().with_bits_set(Self::OVER_RANGE), i32::MAX);
        }

        (self.get_flags(), self.get_value() as i32)
    }

    fn to_f32(&self) -> (Flags, f32) {
        if self.get_value() < f32::MIN.into() {
            return (self.get_flags().with_bits_set(Self::OVER_RANGE), f32::MIN);
        }

        if self.get_value() > f32::MAX.into() {
            return (self.get_flags().with_bits_set(Self::OVER_RANGE), f32::MAX);
        }

        (self.get_flags(), self.get_value() as f32)
    }
}

/// Measurement type corresponding to groups 40 and 42
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct AnalogOutputStatus {
    /// value of the type
    pub value: f64,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Option<Time>,
}

impl AnalogOutputStatus {
    pub fn new(value: f64, flags: Flags, time: Time) -> Self {
        Self {
            value,
            flags,
            time: Some(time),
        }
    }
}
