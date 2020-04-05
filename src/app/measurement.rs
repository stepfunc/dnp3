use crate::app::flags::Flags;
use crate::app::types::{DoubleBit, Timestamp};

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

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Binary {
    pub value: bool,
    pub flags: Flags,
    pub time: Time,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct DoubleBitBinary {
    pub value: DoubleBit,
    pub flags: Flags,
    pub time: Time,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct BinaryOutputStatus {
    pub value: bool,
    pub flags: Flags,
    pub time: Time,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Counter {
    pub value: u32,
    pub flags: Flags,
    pub time: Time,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct FrozenCounter {
    pub value: u32,
    pub flags: Flags,
    pub time: Time,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Analog {
    pub value: f64,
    pub flags: Flags,
    pub time: Time,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct AnalogOutputStatus {
    pub value: f64,
    pub flags: Flags,
    pub time: Time,
}
