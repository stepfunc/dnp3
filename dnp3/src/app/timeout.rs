use std::time::Duration;

/// A wrapper around a std::time::Duration
/// that ensures values are in the range `[1ms .. 1hour]`
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Timeout {
    value: Duration,
}

impl Default for Timeout {
    fn default() -> Self {
        Self {
            value: Duration::from_secs(5),
        }
    }
}

/// Error type returned when a Timeout is constructed with an out-of-range value
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RangeError {
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

    /// try to construct a `Timeout` from a count of seconds
    ///
    /// returns a `RangeError` is < `Timeout::MIN` or > `Timeout::MAX`
    pub fn from_secs(x: u64) -> Result<Self, RangeError> {
        Self::from_duration(Duration::from_secs(x))
    }

    /// try to construct a `Timeout` from a count of milliseconds
    ///
    /// returns a `RangeError` is < `Timeout::MIN` or > `Timeout::MAX`
    pub fn from_millis(x: u64) -> Result<Self, RangeError> {
        Self::from_duration(Duration::from_millis(x))
    }

    /// try to construct a `Timeout` from a `Duration`
    ///
    /// returns a `RangeError` is < `Timeout::MIN` or > `Timeout::MAX`
    pub fn from_duration(value: Duration) -> Result<Self, RangeError> {
        if value < Self::MIN {
            return Err(RangeError::TooSmall(value));
        }

        if value > Self::MAX {
            return Err(RangeError::TooLarge(value));
        }

        Ok(Self { value })
    }

    pub(crate) fn deadline_from_now(self) -> crate::tokio::time::Instant {
        // if this panics due to overflow we have bigger problems than the panic
        // it means the tim value being returned by now() is WAAAY too big
        crate::tokio::time::Instant::now() + self.value
    }
}

impl std::fmt::Display for Timeout {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} ms", self.value.as_millis())
    }
}

impl std::fmt::Display for RangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RangeError::TooSmall(x) => write!(
                f,
                "specified duration ({} ms) smaller than allowed library minimum ({} ms)",
                x.as_millis(),
                Timeout::MIN.as_millis()
            ),
            RangeError::TooLarge(x) => write!(
                f,
                "specified duration ({} ms) larger than allowed library maximum ({} ms)",
                x.as_millis(),
                Timeout::MAX.as_millis()
            ),
        }
    }
}

impl std::error::Error for RangeError {}
