use std::time::Duration;

/// A wrapper around a std::time::Duration that ensures values are in the range `[1ms .. 1hour]`
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "serialization", serde(try_from = "Duration"))]
pub struct Timeout(pub(crate) Duration);

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
    /// returns a `RangeError` is < `Timeout::MIN` or > `Timeout::MAX`
    pub fn from_secs(x: u64) -> Result<Self, TimeoutRangeError> {
        Self::from_duration(Duration::from_secs(x))
    }

    /// try to construct a `Timeout` from a count of milliseconds
    ///
    /// returns a `RangeError` is < `Timeout::MIN` or > `Timeout::MAX`
    pub fn from_millis(x: u64) -> Result<Self, TimeoutRangeError> {
        Self::from_duration(Duration::from_millis(x))
    }

    /// try to construct a `Timeout` from a `Duration`
    ///
    /// returns a `RangeError` is < `Timeout::MIN` or > `Timeout::MAX`
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
        // if this panics due to overflow we have bigger problems than the panic
        // it means the tim value being returned by now() is WAAAY too big
        tokio::time::Instant::now() + self.0
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
