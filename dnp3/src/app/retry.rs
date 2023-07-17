use std::time::Duration;

/// Parameterizes the minimum and maximum delays between retries
/// for a retry strategy based on exponential backoff
#[derive(Copy, Clone, Debug)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct RetryStrategy {
    pub(crate) min_delay: Duration,
    pub(crate) max_delay: Duration,
}

impl RetryStrategy {
    /// construct a `RetryStrategy`
    ///
    /// `min_delay` - the minimum amount of time for the retry
    /// `max_delay` - the maximum amount of time for the retry
    pub fn new(min_delay: Duration, max_delay: Duration) -> Self {
        Self {
            min_delay,
            max_delay,
        }
    }
}

/// Parameterizes connection attempts
#[derive(Copy, Clone, Debug)]
pub struct ConnectStrategy {
    /// Minimum delay between two connection attempts, doubles up to the maximum delay
    pub(crate) min_connect_delay: Duration,
    /// Maximum delay between two connection attempts
    pub(crate) max_connect_delay: Duration,
    /// Delay before attempting a connection after a disconnect
    pub(crate) reconnect_delay: Duration,
}

impl ConnectStrategy {
    /// construct a `ConnectStrategy`
    ///
    /// `min_connect_delay` - Minimum delay between two connection attempts, doubles up to the maximum delay
    /// `max_connect_delay` - Maximum delay between two connection attempts
    /// `reconnect_delay` - Delay before attempting a connection after a disconnect
    pub fn new(
        min_connect_delay: Duration,
        max_connect_delay: Duration,
        reconnect_delay: Duration,
    ) -> Self {
        ConnectStrategy {
            min_connect_delay,
            max_connect_delay,
            reconnect_delay,
        }
    }
}

impl Default for ConnectStrategy {
    fn default() -> Self {
        Self::new(
            Duration::from_secs(1),
            Duration::from_secs(10),
            Duration::from_secs(1),
        )
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self::new(Duration::from_secs(1), Duration::from_secs(10))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ExponentialBackOff {
    strategy: RetryStrategy,
    last: Option<Duration>,
}

impl ExponentialBackOff {
    pub(crate) fn new(strategy: RetryStrategy) -> Self {
        Self {
            strategy,
            last: None,
        }
    }

    pub(crate) fn on_success(&mut self) {
        self.last = None;
    }

    pub(crate) fn on_failure(&mut self) -> Duration {
        match self.last {
            Some(x) => {
                let next = x
                    .checked_mul(2)
                    .unwrap_or(self.strategy.max_delay)
                    .min(self.strategy.max_delay);
                self.last = Some(next);
                next
            }
            None => {
                self.last = Some(self.strategy.min_delay);
                self.strategy.min_delay
            }
        }
    }
}
