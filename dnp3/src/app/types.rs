// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::time::{Duration, SystemTime};

use crate::app::measurement::DoubleBit;
use crate::app::variations::Variation;
use crate::app::QualifierCode;

use scursor::{WriteCursor, WriteError};

/// Wrapper around a u64 count of milliseconds since Unix epoch UTC
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    value: u64,
}

impl Timestamp {
    /// Maximum allowed DNP3 timestamp value (48-bits)
    pub const MAX_VALUE: u64 = 0x0000_FFFF_FFFF_FFFF;

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
        let millis: u64 = x.as_millis().try_into().ok()?;
        let value = self.value.checked_add(millis)?;
        (value <= Self::MAX_VALUE).then_some(Timestamp::new(value))
    }
}

/// Convert days since Unix epoch (1970-01-01) to a proleptic Gregorian `(year, month, day)`.
///
/// Uses Howard Hinnant's `civil_from_days` algorithm over non-negative day counts.
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days.saturating_add(719_468);
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = doe
        .saturating_sub(doe / 1_460)
        .saturating_add(doe / 36_524)
        .saturating_sub(doe / 146_096)
        / 365;
    let y = yoe.saturating_add(era.saturating_mul(400));
    let doy = doe.saturating_sub(
        yoe.saturating_mul(365)
            .saturating_add(yoe / 4)
            .saturating_sub(yoe / 100),
    );
    let mp = doy.saturating_mul(5).saturating_add(2) / 153;
    let day = doy
        .saturating_sub(mp.saturating_mul(153).saturating_add(2) / 5)
        .saturating_add(1);
    let month = if mp < 10 {
        mp.saturating_add(3)
    } else {
        mp.saturating_sub(9)
    };
    let year = if month <= 2 { y.saturating_add(1) } else { y };
    (year, month, day)
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let millis = self.value % 1_000;
        let total_secs = self.value / 1_000;
        let second = total_secs % 60;
        let total_mins = total_secs / 60;
        let minute = total_mins % 60;
        let total_hours = total_mins / 60;
        let hour = total_hours % 24;
        let days = total_hours / 24;

        let (year, month, day) = civil_from_days(days);
        if year > 9999 {
            write!(
                f,
                "+{year}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z"
            )
        } else {
            write!(
                f,
                "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z"
            )
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
            QualifierCode::CountAndPrefix8 => "1-byte count and prefix",
            QualifierCode::CountAndPrefix16 => "2-byte count and prefix",
            QualifierCode::FreeFormat16 => "2-byte free format",
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use proptest::prelude::*;

    fn is_leap_year(year: u64) -> bool {
        (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
    }

    fn days_in_month(year: u64, month: u64) -> u64 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if is_leap_year(year) => 29,
            2 => 28,
            _ => panic!("invalid month {month}"),
        }
    }

    fn next_civil_day((year, month, day): (u64, u64, u64)) -> (u64, u64, u64) {
        if day < days_in_month(year, month) {
            (year, month, day.saturating_add(1))
        } else if month < 12 {
            (year, month.saturating_add(1), 1)
        } else {
            (year.saturating_add(1), 1, 1)
        }
    }

    #[test]
    fn timestamp_display_formatting_works_as_expected() {
        let cases = [
            (0u64, "1970-01-01T00:00:00.000Z"),
            (1, "1970-01-01T00:00:00.001Z"),
            (999, "1970-01-01T00:00:00.999Z"),
            (1_000, "1970-01-01T00:00:01.000Z"),
            (86_399_999, "1970-01-01T23:59:59.999Z"),
            (86_400_000, "1970-01-02T00:00:00.000Z"),
            // 2000 leap year (divisible by 400)
            (951_696_000_000, "2000-02-28T00:00:00.000Z"),
            (951_782_400_000, "2000-02-29T00:00:00.000Z"),
            (951_868_799_999, "2000-02-29T23:59:59.999Z"),
            (951_868_800_000, "2000-03-01T00:00:00.000Z"),
            // 2024 leap year (divisible by 4)
            (1_709_078_400_000, "2024-02-28T00:00:00.000Z"),
            (1_709_164_800_000, "2024-02-29T00:00:00.000Z"),
            (1_709_251_199_999, "2024-02-29T23:59:59.999Z"),
            (1_709_251_200_000, "2024-03-01T00:00:00.000Z"),
            // 2100 non-leap year (divisible by 100, not 400)
            (4_107_456_000_000, "2100-02-28T00:00:00.000Z"),
            (4_107_542_399_999, "2100-02-28T23:59:59.999Z"),
            (4_107_542_400_000, "2100-03-01T00:00:00.000Z"),
            // 4-digit to 5-digit year boundary
            (253_402_300_799_999, "9999-12-31T23:59:59.999Z"),
            (253_402_300_800_000, "+10000-01-01T00:00:00.000Z"),
            // 48-bit maximum
            (
                Timestamp::MAX_VALUE.saturating_sub(1),
                "+10889-08-02T05:31:50.654Z",
            ),
            (Timestamp::MAX_VALUE, "+10889-08-02T05:31:50.655Z"),
            // Masked to 48 bits
            (u64::MAX, "+10889-08-02T05:31:50.655Z"),
        ];

        for (ms, expected) in cases {
            assert_eq!(format!("{}", Timestamp::new(ms)), expected, "ms={ms}");
        }
        assert_eq!(format!("{}", Timestamp::min()), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            format!("{}", Timestamp::max()),
            "+10889-08-02T05:31:50.655Z"
        );
    }

    proptest! {
        #[test]
        fn civil_from_days_advances_by_one_calendar_day(days in 0u64..(Timestamp::MAX_VALUE / 86_400_000)) {
            let today = civil_from_days(days);
            let tomorrow = civil_from_days(days.saturating_add(1));
            prop_assert_eq!(tomorrow, next_civil_day(today));
        }

        #[test]
        fn timestamp_display_produces_valid_rfc3339_millis_utc(ms in 0u64..=Timestamp::MAX_VALUE) {
            let formatted = format!("{}", Timestamp::new(ms));
            let bytes = formatted.as_bytes();
            let (year_str, rest) = if formatted.starts_with('+') {
                prop_assert_eq!(bytes.len(), 26);
                let rest = formatted.strip_prefix('+').unwrap();
                rest.split_at_checked(5).unwrap()
            } else {
                prop_assert_eq!(bytes.len(), 24);
                formatted.split_at_checked(4).unwrap()
            };

            let year: u64 = year_str.parse().unwrap();
            if formatted.starts_with('+') {
                prop_assert!((10_000..=10_889).contains(&year));
            } else {
                prop_assert!((1_970..=9_999).contains(&year));
            }

            let rest_bytes = rest.as_bytes();
            prop_assert_eq!(rest_bytes[0], b'-');
            prop_assert_eq!(rest_bytes[3], b'-');
            prop_assert_eq!(rest_bytes[6], b'T');
            prop_assert_eq!(rest_bytes[9], b':');
            prop_assert_eq!(rest_bytes[12], b':');
            prop_assert_eq!(rest_bytes[15], b'.');
            prop_assert_eq!(rest_bytes[19], b'Z');

            let month: u64 = std::str::from_utf8(&rest_bytes[1..3]).unwrap().parse().unwrap();
            let day: u64 = std::str::from_utf8(&rest_bytes[4..6]).unwrap().parse().unwrap();
            let hour: u64 = std::str::from_utf8(&rest_bytes[7..9]).unwrap().parse().unwrap();
            let minute: u64 = std::str::from_utf8(&rest_bytes[10..12]).unwrap().parse().unwrap();
            let second: u64 = std::str::from_utf8(&rest_bytes[13..15]).unwrap().parse().unwrap();
            let millis: u64 = std::str::from_utf8(&rest_bytes[16..19]).unwrap().parse().unwrap();

            prop_assert!((1..=12).contains(&month));
            prop_assert!((1..=days_in_month(year, month)).contains(&day));
            prop_assert!(hour <= 23);
            prop_assert!(minute <= 59);
            prop_assert!(second <= 59);
            prop_assert!(millis <= 999);
            prop_assert_eq!(millis, ms % 1_000);
        }
    }
}
