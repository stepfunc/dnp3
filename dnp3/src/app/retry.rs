use std::time::Duration;

/// Parameterizes the minimum and maximum delays between retries
/// for a retry strategy based on exponential backoff
#[derive(Copy, Clone, Debug)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "serialization", serde(from = "RetryStrategyFields"))]
pub struct RetryStrategy {
    pub(crate) min_delay: Duration,
    pub(crate) max_delay: Duration,
}

impl RetryStrategy {
    /// smallest delay a `RetryStrategy` uses
    const MIN_DELAY: Duration = Duration::from_millis(1);

    /// construct a `RetryStrategy`
    ///
    /// `min_delay` - the minimum amount of time for the retry
    /// `max_delay` - the maximum amount of time for the retry
    ///
    /// If `min_delay` is greater than `max_delay`, the two are swapped. Both delays are raised to
    /// at least 1 millisecond so that retries never happen without a delay.
    pub fn new(min_delay: Duration, max_delay: Duration) -> Self {
        let (min_delay, max_delay) = if min_delay <= max_delay {
            (min_delay, max_delay)
        } else {
            (max_delay, min_delay)
        };
        Self {
            min_delay: min_delay.max(Self::MIN_DELAY),
            max_delay: max_delay.max(Self::MIN_DELAY),
        }
    }
}

/// serialized form of [`RetryStrategy`], normalized by [`RetryStrategy::new`] when deserialized
#[cfg(feature = "serialization")]
#[derive(serde::Deserialize)]
#[serde(rename = "RetryStrategy")]
struct RetryStrategyFields {
    min_delay: Duration,
    max_delay: Duration,
}

#[cfg(feature = "serialization")]
impl From<RetryStrategyFields> for RetryStrategy {
    fn from(value: RetryStrategyFields) -> Self {
        Self::new(value.min_delay, value.max_delay)
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

#[cfg(test)]
mod test {
    use super::*;

    fn delays(strategy: RetryStrategy) -> Vec<Duration> {
        let mut backoff = ExponentialBackOff::new(strategy);
        (0..4).map(|_| backoff.on_failure()).collect()
    }

    #[test]
    fn zero_min_delay_backs_off_from_one_millisecond() {
        let strategy = RetryStrategy::new(Duration::ZERO, Duration::from_millis(5));
        assert_eq!(
            delays(strategy),
            [1, 2, 4, 5].map(Duration::from_millis).to_vec()
        );
    }

    #[test]
    fn zero_delays_are_raised_to_one_millisecond() {
        let strategy = RetryStrategy::new(Duration::ZERO, Duration::ZERO);
        assert_eq!(strategy.min_delay, Duration::from_millis(1));
        assert_eq!(strategy.max_delay, Duration::from_millis(1));
    }

    #[test]
    fn reversed_delays_are_swapped() {
        let strategy = RetryStrategy::new(Duration::from_secs(10), Duration::from_secs(1));
        assert_eq!(strategy.min_delay, Duration::from_secs(1));
        assert_eq!(strategy.max_delay, Duration::from_secs(10));
        assert_eq!(
            delays(strategy),
            [1, 2, 4, 8].map(Duration::from_secs).to_vec()
        );
    }

    #[test]
    #[cfg(feature = "serialization")]
    fn serialization_round_trips_in_the_same_format() {
        let json = r#"{"min_delay":{"secs":1,"nanos":0},"max_delay":{"secs":10,"nanos":0}}"#;
        let strategy = RetryStrategy::default();
        assert_eq!(serde_json::to_string(&strategy).unwrap(), json);
        let strategy: RetryStrategy = serde_json::from_str(json).unwrap();
        assert_eq!(strategy.min_delay, Duration::from_secs(1));
        assert_eq!(strategy.max_delay, Duration::from_secs(10));
    }

    #[test]
    #[cfg(feature = "serialization")]
    fn deserialization_normalizes_delays() {
        let strategy: RetryStrategy = serde_json::from_str(
            r#"{"min_delay":{"secs":10,"nanos":0},"max_delay":{"secs":0,"nanos":0}}"#,
        )
        .unwrap();
        assert_eq!(strategy.min_delay, Duration::from_millis(1));
        assert_eq!(strategy.max_delay, Duration::from_secs(10));
    }

    #[test]
    #[cfg(feature = "serialization")]
    fn deserializes_as_a_struct_named_retry_strategy() {
        use serde::de::{Error, Visitor};
        use serde::Deserialize;

        #[derive(Default)]
        struct StructName(Option<&'static str>);

        impl<'de> serde::Deserializer<'de> for &mut StructName {
            type Error = serde::de::value::Error;

            fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
                Err(Error::custom("not a struct"))
            }

            fn deserialize_struct<V: Visitor<'de>>(
                self,
                name: &'static str,
                _: &'static [&'static str],
                _: V,
            ) -> Result<V::Value, Self::Error> {
                self.0 = Some(name);
                Err(Error::custom("name recorded"))
            }

            serde::forward_to_deserialize_any! {
                bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
                byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map enum
                identifier ignored_any
            }
        }

        let mut name = StructName::default();
        RetryStrategy::deserialize(&mut name).unwrap_err();
        assert_eq!(name.0, Some("RetryStrategy"));
    }
}
