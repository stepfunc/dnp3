use std::time::Duration;

/// A wrapper around a std::time::Duration
/// that ensures values are in the range `[1ms .. 1hour]`
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Timeout {
    value: Duration,
}

/// Error type returned when a Timeout is constructed with an out-of-range value
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RangeError {
    TooSmall(Duration), // the applied value and the minimum value
    TooLarge(Duration), // the maximum value, and the maximum value
}

impl Timeout {
    const MIN: Duration = Duration::from_millis(1);
    const MAX: Duration = Duration::from_secs(60 * 60); // one hour

    pub fn from_secs(x: u64) -> Result<Self, RangeError> {
        Self::from_duration(Duration::from_secs(x))
    }

    pub fn from_millis(x: u64) -> Result<Self, RangeError> {
        Self::from_duration(Duration::from_millis(x))
    }

    pub fn from_duration(value: Duration) -> Result<Self, RangeError> {
        if value < Self::MIN {
            return Err(RangeError::TooSmall(value));
        }

        if value > Self::MAX {
            return Err(RangeError::TooLarge(value));
        }

        Ok(Self { value })
    }

    pub(crate) fn from_now(&self) -> tokio::time::Instant {
        // if this panics due to overflow we have bigger problems than the panic
        // it means the tim value being returned by now() is WAAAY too big
        tokio::time::Instant::now() + self.value
    }
}

impl std::fmt::Display for Timeout {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} ms", self.value.as_millis())
    }
}
