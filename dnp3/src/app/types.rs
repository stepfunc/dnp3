use std::time::{Duration, SystemTime};

use crate::app::measurement::DoubleBit;
use crate::app::variations::Variation;
use crate::app::QualifierCode;

use chrono::{DateTime, SecondsFormat, TimeZone, Utc};
use scursor::{WriteCursor, WriteError};

/// Wrapper around a u64 count of milliseconds since Unix epoch UTC
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    value: u64,
}

impl Timestamp {
    /// Maximum allowed DNP3 timestamp value (48-bits)
    pub const MAX_VALUE: u64 = 0x0000_FFFF_FFFF_FFFF;
    pub(crate) const OUT_OF_RANGE: &'static str = "<out of range>";

    /// Create a timestamp from a count of milliseconds since epoch
    pub const fn new(value: u64) -> Self {
        Self {
            value: value & Self::MAX_VALUE,
        }
    }

    /// Minimum valid timestamp
    pub const fn min() -> Self {
        Self::zero()
    }

    /// Timestamp value of zero corresponding to the epoch
    pub const fn zero() -> Self {
        Self::new(0)
    }

    /// Maximum valid timestamp
    pub fn max() -> Self {
        Self::new(Self::MAX_VALUE)
    }

    /// Attempt to create a Timestamp from a SystemTime
    pub fn try_from_system_time(system_time: SystemTime) -> Option<Timestamp> {
        Some(Timestamp::new(
            u64::try_from(
                system_time
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .ok()?
                    .as_millis(),
            )
            .ok()?,
        ))
    }

    /// Attempt to create a `DateTime<Utc>` from a Timestamp
    pub fn to_datetime_utc(self) -> Option<DateTime<Utc>> {
        Utc.timestamp_millis_opt(self.value as i64).single()
    }

    /// Retrieve the raw u64 value
    pub fn raw_value(&self) -> u64 {
        self.value
    }

    pub(crate) fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u48_le(self.value)
    }

    pub(crate) fn read(cursor: &mut scursor::ReadCursor) -> Result<Self, scursor::ReadError> {
        Ok(Self {
            value: cursor.read_u48_le()?,
        })
    }

    pub(crate) fn checked_add(self, x: Duration) -> Option<Timestamp> {
        // safe from overflow since self.value cannot possibly be larger than MAX
        let max_add = Self::MAX_VALUE - self.value;
        let millis = x.as_millis();
        if millis > max_add as u128 {
            return None;
        }
        Some(Timestamp::new(self.value + millis as u64))
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.to_datetime_utc() {
            Some(x) => write!(f, "{}", x.to_rfc3339_opts(SecondsFormat::Millis, true)),
            None => f.write_str(Timestamp::OUT_OF_RANGE),
        }
    }
}

pub(crate) struct BitPair {
    pub(crate) high: bool,
    pub(crate) low: bool,
}

impl BitPair {
    pub(crate) fn new(high: bool, low: bool) -> Self {
        Self { high, low }
    }
}

impl DoubleBit {
    pub(crate) fn from(high: bool, low: bool) -> Self {
        match (high, low) {
            (false, false) => DoubleBit::Intermediate,
            (false, true) => DoubleBit::DeterminedOff,
            (true, false) => DoubleBit::DeterminedOn,
            (true, true) => DoubleBit::Indeterminate,
        }
    }

    pub(crate) fn to_bit_pair(self) -> BitPair {
        match self {
            DoubleBit::Intermediate => BitPair::new(false, false),
            DoubleBit::DeterminedOff => BitPair::new(false, true),
            DoubleBit::DeterminedOn => BitPair::new(true, false),
            DoubleBit::Indeterminate => BitPair::new(true, true),
        }
    }

    pub(crate) fn to_byte(self) -> u8 {
        match self {
            DoubleBit::Intermediate => 0b00,
            DoubleBit::DeterminedOff => 0b01,
            DoubleBit::DeterminedOn => 0b10,
            DoubleBit::Indeterminate => 0b11,
        }
    }
}

impl std::fmt::Display for DoubleBit {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::fmt::Display for Variation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let (g, v) = self.to_group_and_var();
        write!(f, "g{g}v{v}")
    }
}

impl QualifierCode {
    pub(crate) fn description(self) -> &'static str {
        match self {
            QualifierCode::AllObjects => "all objects",
            QualifierCode::Range8 => "1-byte start/stop",
            QualifierCode::Range16 => "2-byte start/stop",
            QualifierCode::Count8 => "1-byte count of objects",
            QualifierCode::Count16 => "2-byte count of objects",
            QualifierCode::CountAndPrefix8 => "1-byte count of objects",
            QualifierCode::CountAndPrefix16 => "2-byte count of objects",
            QualifierCode::FreeFormat16 => "2-byte free format",
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn conversion_from_timestamp_to_datetime_utc_cannot_overflow() {
        let timestamp = Timestamp::new(std::u64::MAX);
        timestamp.to_datetime_utc();
    }

    #[test]
    fn timestamp_display_formatting_works_as_expected() {
        assert_eq!(format!("{}", Timestamp::min()), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            format!("{}", Timestamp::max()),
            "+10889-08-02T05:31:50.655Z"
        );
    }
}
