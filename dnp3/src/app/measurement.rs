use crate::app::flags::Flags;
use crate::app::types::{DoubleBit, Timestamp};

/// A DNP3 time value that may be Invalid, Synchronized, or NotSynchronized
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Time {
    /// The timestamp is UTC synchronized at the remote device
    Synchronized(Timestamp),
    /// The device indicates the timestamp may be not be synchronized
    NotSynchronized(Timestamp),
    /// Timestamp is not valid, ignore the value and use a local timestamp
    Invalid,
}

impl Time {
    pub(crate) fn checked_add(self, x: u16) -> Self {
        match self {
            Time::Invalid => Time::Invalid,
            Time::Synchronized(ts) => match ts.checked_add(x) {
                Some(x) => Time::Synchronized(x),
                None => Time::Invalid,
            },
            Time::NotSynchronized(ts) => match ts.checked_add(x) {
                Some(x) => Time::NotSynchronized(x),
                None => Time::Invalid,
            },
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
    pub time: Time,
}

/// Measurement type corresponding to groups 3 and 4
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct DoubleBitBinary {
    /// value of the type
    pub value: DoubleBit,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Time,
}

/// Measurement type corresponding to groups 10 and 11
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct BinaryOutputStatus {
    /// value of the type
    pub value: bool,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Time,
}

/// Measurement type corresponding to groups 20 and 22
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Counter {
    /// value of the type
    pub value: u32,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Time,
}

/// Measurement type corresponding to groups 21 and 23
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct FrozenCounter {
    /// value of the type
    pub value: u32,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Time,
}

/// Measurement type corresponding to groups 30 and 32
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Analog {
    /// value of the type
    pub value: f64,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Time,
}

/// Measurement type corresponding to groups 40 and 42
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct AnalogOutputStatus {
    /// value of the type
    pub value: f64,
    /// associated flags
    pub flags: Flags,
    /// associated time
    pub time: Time,
}
