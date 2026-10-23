// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::time::Duration;

/// A wrapper around a std::time::Duration that ensures values are in the range `[1ms .. 1hour]`
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "serialization", serde(try_from = "Duration"))]
pub struct Timeout(Duration);

impl From<Timeout> for Duration {
    fn from(value: Timeout) -> Self {
        value.0
    }
}

impl Default for Timeout {
    fn default() -> Self {
        Self(Duration::from_secs(5))
    }
}

impl TryFrom<Duration> for Timeout {
    type Error = TimeoutRangeError;

    fn try_from(value: Duration) -> Result<Self, Self::Error> {
        Timeout::from_duration(value)
    }
}

/// Error type returned when a Timeout is constructed with an out-of-range value
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TimeoutRangeError {
    /// value smaller than library allowed minimum
    TooSmall(Duration),
    /// value larger than library allowed maximum
    TooLarge(Duration),
}

impl Timeout {
    /// minimum allowed timeout value as a duration
    pub const MIN: Duration = Duration::from_millis(1);
    /// maximum allowed timeout value as a duration
    pub const MAX: Duration = Duration::from_secs(60 * 60); // one hour

    /// construct from a duration, saturating at the minimum and maximum
    pub fn saturating(value: Duration) -> Self {
        if value < Self::MIN {
            return Self(Self::MIN);
        }

        if value > Self::MAX {
            return Self(Self::MAX);
        }

        Self(value)
    }

    /// try to construct a `Timeout` from a count of seconds
    ///
    /// returns a [`TimeoutRangeError`] if it is < [`Timeout::MIN`] or > [`Timeout::MAX`]
    pub fn from_secs(x: u64) -> Result<Self, TimeoutRangeError> {
        Self::from_duration(Duration::from_secs(x))
    }

    /// try to construct a `Timeout` from a count of milliseconds
    ///
    /// returns a [`TimeoutRangeError`] if it is < [`Timeout::MIN`] or > [`Timeout::MAX`]
    pub fn from_millis(x: u64) -> Result<Self, TimeoutRangeError> {
        Self::from_duration(Duration::from_millis(x))
    }

    /// try to construct a `Timeout` from a `Duration`
    ///
    /// returns a [`TimeoutRangeError`] if it is < [`Timeout::MIN`] or > [`Timeout::MAX`]
    pub fn from_duration(value: Duration) -> Result<Self, TimeoutRangeError> {
        if value < Self::MIN {
            return Err(TimeoutRangeError::TooSmall(value));
        }

        if value > Self::MAX {
            return Err(TimeoutRangeError::TooLarge(value));
        }

        Ok(Self(value))
    }

    pub(crate) fn deadline_from_now(self) -> tokio::time::Instant {
        crate::util::deadline::deadline(tokio::time::Instant::now(), self.0)
    }
}

impl std::fmt::Display for Timeout {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} ms", self.0.as_millis())
    }
}

impl std::fmt::Display for TimeoutRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TimeoutRangeError::TooSmall(x) => write!(
                f,
                "specified duration ({} ms) smaller than allowed library minimum ({} ms)",
                x.as_millis(),
                Timeout::MIN.as_millis()
            ),
            TimeoutRangeError::TooLarge(x) => write!(
                f,
                "specified duration ({} ms) larger than allowed library maximum ({} ms)",
                x.as_millis(),
                Timeout::MAX.as_millis()
            ),
        }
    }
}

impl std::error::Error for TimeoutRangeError {}

#[cfg(test)]
mod test {
    use super::*;

    const JUST_BELOW_MIN: Duration = Duration::from_nanos(999_999);
    const JUST_ABOVE_MAX: Duration = Duration::from_nanos(3_600_000_000_001);

    #[test]
    fn range_is_one_millisecond_to_one_hour() {
        assert_eq!(Timeout::MIN, Duration::from_nanos(1_000_000));
        assert_eq!(Timeout::MAX, Duration::from_nanos(3_600_000_000_000));
    }

    #[test]
    fn from_duration_accepts_only_the_closed_range() {
        let cases = [
            (
                Duration::ZERO,
                Err(TimeoutRangeError::TooSmall(Duration::ZERO)),
            ),
            (
                JUST_BELOW_MIN,
                Err(TimeoutRangeError::TooSmall(JUST_BELOW_MIN)),
            ),
            (Timeout::MIN, Ok(Timeout::MIN)),
            (Timeout::MAX, Ok(Timeout::MAX)),
            (
                JUST_ABOVE_MAX,
                Err(TimeoutRangeError::TooLarge(JUST_ABOVE_MAX)),
            ),
            (
                Duration::MAX,
                Err(TimeoutRangeError::TooLarge(Duration::MAX)),
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(
                Timeout::from_duration(input).map(Duration::from),
                expected,
                "{input:?}"
            );
            assert_eq!(
                Timeout::try_from(input).map(Duration::from),
                expected,
                "{input:?}"
            );
        }
    }

    #[test]
    fn from_millis_and_from_secs_check_the_range() {
        assert_eq!(
            Timeout::from_millis(0),
            Err(TimeoutRangeError::TooSmall(Duration::ZERO))
        );
        assert_eq!(
            Timeout::from_millis(1).map(Duration::from),
            Ok(Timeout::MIN)
        );
        assert_eq!(
            Timeout::from_secs(3600).map(Duration::from),
            Ok(Timeout::MAX)
        );
        assert_eq!(
            Timeout::from_secs(3601),
            Err(TimeoutRangeError::TooLarge(Duration::from_secs(3601)))
        );
    }

    #[test]
    fn saturating_clamps_to_the_range() {
        let cases = [
            (Duration::ZERO, Timeout::MIN),
            (JUST_BELOW_MIN, Timeout::MIN),
            (Timeout::MIN, Timeout::MIN),
            (Duration::from_secs(5), Duration::from_secs(5)),
            (Timeout::MAX, Timeout::MAX),
            (JUST_ABOVE_MAX, Timeout::MAX),
            (Duration::MAX, Timeout::MAX),
        ];

        for (input, expected) in cases {
            assert_eq!(
                Duration::from(Timeout::saturating(input)),
                expected,
                "{input:?}"
            );
        }
    }

    #[cfg(feature = "serialization")]
    #[test]
    fn deserialization_checks_the_range() {
        let cases = [
            (Duration::ZERO, None),
            (JUST_BELOW_MIN, None),
            (Timeout::MIN, Some(Timeout::MIN)),
            (Timeout::MAX, Some(Timeout::MAX)),
            (JUST_ABOVE_MAX, None),
        ];

        for (input, expected) in cases {
            let json = serde_json::to_string(&input).unwrap();
            let parsed = serde_json::from_str::<Timeout>(&json)
                .ok()
                .map(Duration::from);
            assert_eq!(parsed, expected, "{input:?}");
        }
    }

    #[cfg(feature = "serialization")]
    #[test]
    fn serializes_as_the_inner_duration() {
        let timeout = Timeout::from_millis(1500).unwrap();
        assert_eq!(
            serde_json::to_value(timeout).unwrap(),
            serde_json::to_value(Duration::from_millis(1500)).unwrap()
        );
    }
}
