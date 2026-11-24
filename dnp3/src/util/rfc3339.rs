// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Standalone RFC 3339 UTC formatting for millisecond timestamps since the Unix epoch.

const EXTENDED_YEAR_DIVISORS: [u64; 9] = [
    100_000_000,
    10_000_000,
    1_000_000,
    100_000,
    10_000,
    1_000,
    100,
    10,
    1,
];

/// Convert days since Unix epoch (1970-01-01) to a proleptic Gregorian `(year, month, day)`.
///
/// Uses Howard Hinnant's `civil_from_days` algorithm over non-negative day counts.
/// Exact for every day count reachable from a `u64` millisecond timestamp (up to
/// `u64::MAX / 86_400_000`, year `584_556_019`).
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

/// Fixed-capacity ASCII buffer holding a formatted RFC 3339 UTC millisecond timestamp.
///
/// The maximum length for any `u64` millisecond count is 30 bytes
/// (`+584556019-04-03T14:25:51.615Z`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct FormattedUtcMillis {
    buf: [u8; 30],
    len: usize,
}

impl FormattedUtcMillis {
    /// Format a count of milliseconds since the Unix epoch (`1970-01-01T00:00:00.000Z`)
    /// into a stack-allocated ASCII buffer.
    pub(crate) fn new(millis_since_epoch: u64) -> Self {
        let millis = millis_since_epoch % 1_000;
        let total_secs = millis_since_epoch / 1_000;
        let second = total_secs % 60;
        let total_mins = total_secs / 60;
        let minute = total_mins % 60;
        let total_hours = total_mins / 60;
        let hour = total_hours % 24;
        let days = total_hours / 24;

        let (year, month, day) = civil_from_days(days);

        let mut out = Self {
            buf: [0; 30],
            len: 0,
        };
        if year > 9_999 {
            out.push(b'+');
            out.push_extended_year(year);
        } else {
            out.push_four_digits(year);
        }
        out.push(b'-');
        out.push_two_digits(month);
        out.push(b'-');
        out.push_two_digits(day);
        out.push(b'T');
        out.push_two_digits(hour);
        out.push(b':');
        out.push_two_digits(minute);
        out.push(b':');
        out.push_two_digits(second);
        out.push(b'.');
        out.push_three_digits(millis);
        out.push(b'Z');
        out
    }

    /// Borrow the formatted timestamp as ASCII bytes.
    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.buf.get(..self.len).unwrap_or(&self.buf)
    }

    /// Borrow the formatted timestamp as a UTF-8 string slice.
    pub(crate) fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes()).unwrap_or("")
    }

    fn push(&mut self, byte: u8) {
        if let Some(slot) = self.buf.get_mut(self.len) {
            *slot = byte;
            self.len = self.len.saturating_add(1);
        }
    }

    fn push_digit(&mut self, value: u64) {
        let digit = u8::try_from(value % 10).unwrap_or(0);
        self.push(b'0'.saturating_add(digit));
    }

    fn push_two_digits(&mut self, value: u64) {
        self.push_digit(value / 10);
        self.push_digit(value);
    }

    fn push_three_digits(&mut self, value: u64) {
        self.push_digit(value / 100);
        self.push_digit(value / 10);
        self.push_digit(value);
    }

    fn push_four_digits(&mut self, value: u64) {
        self.push_digit(value / 1_000);
        self.push_digit(value / 100);
        self.push_digit(value / 10);
        self.push_digit(value);
    }

    fn push_extended_year(&mut self, year: u64) {
        let mut started = false;
        for divisor in EXTENDED_YEAR_DIVISORS {
            let quotient = year.checked_div(divisor).unwrap_or(0);
            if started || quotient > 0 {
                started = true;
                self.push_digit(quotient);
            }
        }
    }
}

/// Format a count of milliseconds since the Unix epoch (`1970-01-01T00:00:00.000Z`) as a
/// UTC RFC 3339 timestamp with millisecond precision.
///
/// Four-digit years (`1970`..=`9999`) are formatted as `YYYY-MM-DDTHH:MM:SS.mmmZ`.
/// Years above `9999` (up to `584556019` at `u64::MAX`) are prefixed with `+` as
/// `+YYYYY...-MM-DDTHH:MM:SS.mmmZ`.
pub(crate) fn format_utc_millis(
    millis_since_epoch: u64,
    f: &mut core::fmt::Formatter<'_>,
) -> core::fmt::Result {
    f.write_str(FormattedUtcMillis::new(millis_since_epoch).as_str())
}

#[cfg(test)]
mod tests {
    use core::fmt;

    use proptest::prelude::*;

    use super::*;

    const MAX_DAYS_FROM_U64_MILLIS: u64 = u64::MAX / 86_400_000;
    const YEAR_10000_MILLIS: u64 = 253_402_300_800_000;
    const U48_MAX: u64 = 0x0000_FFFF_FFFF_FFFF;

    struct UtcMillis(u64);

    impl fmt::Display for UtcMillis {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            format_utc_millis(self.0, f)
        }
    }

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
    fn format_utc_millis_matches_known_boundaries() {
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
            (YEAR_10000_MILLIS, "+10000-01-01T00:00:00.000Z"),
            // 48-bit maximum
            (U48_MAX.saturating_sub(1), "+10889-08-02T05:31:50.654Z"),
            (U48_MAX, "+10889-08-02T05:31:50.655Z"),
            // Full u64 maximum
            (u64::MAX.saturating_sub(1), "+584556019-04-03T14:25:51.614Z"),
            (u64::MAX, "+584556019-04-03T14:25:51.615Z"),
        ];

        for (ms, expected) in cases {
            assert_eq!(format!("{}", UtcMillis(ms)), expected, "ms={ms}");
        }
    }

    fn day_strategy() -> impl Strategy<Value = u64> {
        prop_oneof![0u64..(U48_MAX / 86_400_000), 0u64..MAX_DAYS_FROM_U64_MILLIS]
    }

    fn millis_strategy() -> impl Strategy<Value = u64> {
        prop_oneof![
            0u64..YEAR_10000_MILLIS,
            YEAR_10000_MILLIS..=U48_MAX,
            any::<u64>(),
        ]
    }

    proptest! {
        #[test]
        fn civil_from_days_advances_by_one_calendar_day(days in day_strategy()) {
            let today = civil_from_days(days);
            let tomorrow = civil_from_days(days.saturating_add(1));
            prop_assert_eq!(tomorrow, next_civil_day(today));
        }

        #[test]
        fn format_utc_millis_produces_valid_rfc3339_millis_utc(ms in millis_strategy()) {
            let formatted = format!("{}", UtcMillis(ms));
            let (year_part, rest) = formatted.split_once('-').unwrap();

            let year: u64 = if let Some(extended) = year_part.strip_prefix('+') {
                prop_assert!((5..=9).contains(&extended.len()));
                prop_assert!(!extended.starts_with('0'));
                let year: u64 = extended.parse().unwrap();
                prop_assert!((10_000..=584_556_019).contains(&year));
                year
            } else {
                prop_assert_eq!(year_part.len(), 4);
                let year: u64 = year_part.parse().unwrap();
                prop_assert!((1_970..=9_999).contains(&year));
                year
            };

            let rest_bytes = rest.as_bytes();
            prop_assert_eq!(rest_bytes.len(), 19);
            prop_assert_eq!(rest_bytes[2], b'-');
            prop_assert_eq!(rest_bytes[5], b'T');
            prop_assert_eq!(rest_bytes[8], b':');
            prop_assert_eq!(rest_bytes[11], b':');
            prop_assert_eq!(rest_bytes[14], b'.');
            prop_assert_eq!(rest_bytes[18], b'Z');

            let month: u64 = core::str::from_utf8(&rest_bytes[0..2]).unwrap().parse().unwrap();
            let day: u64 = core::str::from_utf8(&rest_bytes[3..5]).unwrap().parse().unwrap();
            let hour: u64 = core::str::from_utf8(&rest_bytes[6..8]).unwrap().parse().unwrap();
            let minute: u64 = core::str::from_utf8(&rest_bytes[9..11]).unwrap().parse().unwrap();
            let second: u64 = core::str::from_utf8(&rest_bytes[12..14]).unwrap().parse().unwrap();
            let millis: u64 = core::str::from_utf8(&rest_bytes[15..18]).unwrap().parse().unwrap();

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
