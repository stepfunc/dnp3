use std::time::Duration;

/// Combines a `RetryStrategy` with an optional delay before reconnecting after a disconnection
#[derive(Copy, Clone, Debug)]
pub struct ReconnectStrategy {
    /// Retry strategy when used between attempts to
    /// establish a connection
    pub retry_strategy: RetryStrategy,
    /// Optional delay to wait before attempting a new connection
    /// when an existing connection is lost
    pub reconnect_delay: Option<Duration>,
}

impl ReconnectStrategy {
    pub fn new(retry_strategy: RetryStrategy, reconnect_delay: Option<Duration>) -> Self {
        Self {
            retry_strategy,
            reconnect_delay,
        }
    }
}

impl From<RetryStrategy> for ReconnectStrategy {
    fn from(from: RetryStrategy) -> Self {
        Self::new(from, None)
    }
}

impl Default for ReconnectStrategy {
    fn default() -> Self {
        Self::new(RetryStrategy::default(), None)
    }
}

/// Parameterizes the minimum and maximum delays between retries
/// for a retry strategy based on exponential backoff
#[derive(Copy, Clone, Debug)]
pub struct RetryStrategy {
    pub(crate) min_delay: Duration,
    pub(crate) max_delay: Duration,
}

impl RetryStrategy {
    pub fn new(min_delay: Duration, max_delay: Duration) -> Self {
        Self {
            min_delay,
            max_delay,
        }
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
