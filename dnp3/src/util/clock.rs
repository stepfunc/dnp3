// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use core::time::Duration;

/// Monotonic, runtime-independent point in time measured from a clock's epoch.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MonotonicTime(Duration);

impl MonotonicTime {
    /// Construct a monotonic time at `elapsed` since the clock's epoch.
    pub(crate) const fn from_elapsed(elapsed: Duration) -> Self {
        Self(elapsed)
    }

    /// Add `duration`, or `None` if the result can't be represented.
    pub(crate) fn checked_add(self, duration: Duration) -> Option<Self> {
        self.0.checked_add(duration).map(Self)
    }

    /// Add `duration`, saturating at [`Duration::MAX`] instead of overflowing.
    pub(crate) fn saturating_add(self, duration: Duration) -> Self {
        Self(self.0.saturating_add(duration))
    }

    /// Duration elapsed since `earlier`, or `None` if `self < earlier`.
    pub(crate) fn checked_duration_since(self, earlier: Self) -> Option<Duration> {
        self.0.checked_sub(earlier.0)
    }

    /// Duration elapsed since `earlier`, or [`Duration::ZERO`] if `self <= earlier`.
    pub(crate) fn saturating_duration_since(self, earlier: Self) -> Duration {
        self.0.saturating_sub(earlier.0)
    }
}

/// Monotonic time source for protocol state machines.
pub(crate) trait MonotonicClock {
    /// Current monotonic time.
    fn now(&self) -> MonotonicTime;

    /// Compute a monotonic deadline `delay` from now, saturating at [`Duration::MAX`].
    fn deadline(&self, delay: Duration) -> MonotonicTime {
        self.now().saturating_add(delay)
    }

    /// Returns `true` once the clock has reached or passed `target` (`now >= target`).
    fn has_reached(&self, target: MonotonicTime) -> bool {
        self.now() >= target
    }
}

impl<C: MonotonicClock + ?Sized> MonotonicClock for &C {
    fn now(&self) -> MonotonicTime {
        (**self).now()
    }
}

/// [`MonotonicClock`] backed by [`tokio::time::Instant`], capturing its epoch once at creation.
#[derive(Copy, Clone, Debug)]
pub(crate) struct TokioClock {
    epoch: tokio::time::Instant,
}

impl TokioClock {
    /// Capture the current Tokio instant as the epoch for this clock.
    pub(crate) fn new() -> Self {
        Self {
            epoch: tokio::time::Instant::now(),
        }
    }

    fn elapsed(self, instant: tokio::time::Instant) -> MonotonicTime {
        MonotonicTime::from_elapsed(instant.saturating_duration_since(self.epoch))
    }

    /// Convert a [`MonotonicTime`] into a [`tokio::time::Instant`] sleep deadline,
    /// clamping excessively large remaining delays so the deadline can always be represented.
    pub(crate) fn to_tokio_instant(self, target: MonotonicTime) -> tokio::time::Instant {
        let now = tokio::time::Instant::now();
        let remaining = target.saturating_duration_since(self.elapsed(now));
        crate::util::deadline::deadline(now, remaining)
    }

    /// Sleep until `deadline`, or stay pending forever when `deadline` is `None`.
    pub(crate) async fn sleep_until(self, deadline: Option<MonotonicTime>) {
        match deadline {
            Some(target) => tokio::time::sleep_until(self.to_tokio_instant(target)).await,
            None => std::future::pending().await,
        }
    }
}

impl MonotonicClock for TokioClock {
    fn now(&self) -> MonotonicTime {
        self.elapsed(tokio::time::Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monotonic_time_ordering_and_arithmetic() {
        let t0 = MonotonicTime::from_elapsed(Duration::ZERO);
        let t1 = t0.saturating_add(Duration::from_secs(5));
        let t2 = t1.saturating_add(Duration::from_millis(250));

        assert!(t0 < t1);
        assert!(t1 < t2);
        assert_eq!(t1.checked_duration_since(t0), Some(Duration::from_secs(5)));
        assert_eq!(
            t2.checked_duration_since(t1),
            Some(Duration::from_millis(250))
        );
        assert_eq!(t1.checked_duration_since(t1), Some(Duration::ZERO));
        assert_eq!(t0.checked_duration_since(t1), None);

        assert_eq!(
            t2.saturating_duration_since(t0),
            Duration::from_millis(5250)
        );
        assert_eq!(t0.saturating_duration_since(t2), Duration::ZERO);
    }

    #[test]
    fn monotonic_time_saturates_at_duration_max() {
        let max = MonotonicTime::from_elapsed(Duration::MAX);
        let t = MonotonicTime::from_elapsed(Duration::from_secs(10));

        assert_eq!(t.saturating_add(Duration::MAX), max);
        assert_eq!(max.saturating_add(Duration::from_nanos(1)), max);
        assert_eq!(max.checked_duration_since(max), Some(Duration::ZERO));
        assert_eq!(
            max.checked_duration_since(t),
            Duration::MAX.checked_sub(Duration::from_secs(10))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn tokio_clock_advances_monotonically_from_epoch() {
        let clock = TokioClock::new();
        assert_eq!(clock.now(), MonotonicTime::from_elapsed(Duration::ZERO));

        tokio::time::advance(Duration::from_secs(12)).await;
        assert_eq!(
            clock.now(),
            MonotonicTime::from_elapsed(Duration::from_secs(12))
        );

        let clock_ref: &dyn MonotonicClock = &clock;
        assert_eq!(
            clock_ref.now(),
            MonotonicTime::from_elapsed(Duration::from_secs(12))
        );

        tokio::time::advance(Duration::from_millis(345)).await;
        assert_eq!(
            clock.now(),
            MonotonicTime::from_elapsed(Duration::from_millis(12_345))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn tokio_clock_converts_and_clamps_deadlines() {
        let clock = TokioClock::new();
        tokio::time::advance(Duration::from_secs(10)).await;

        let tokio_now = tokio::time::Instant::now();
        let now = clock.now();

        // ordinary future deadline
        let delay = Duration::from_secs(5);
        assert_eq!(
            clock.to_tokio_instant(now.saturating_add(delay)),
            tokio_now.checked_add(delay).unwrap()
        );

        // past and current deadlines resolve to now
        assert_eq!(
            clock.to_tokio_instant(MonotonicTime::from_elapsed(Duration::from_secs(3))),
            tokio_now
        );
        assert_eq!(clock.to_tokio_instant(now), tokio_now);

        // Duration::MAX saturates and clamps without panicking
        let max_target = now.saturating_add(Duration::MAX);
        assert_eq!(
            clock.to_tokio_instant(max_target),
            crate::util::deadline::deadline(tokio_now, Duration::MAX)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn tokio_clock_handles_deadlines_past_max_delay_uptime() {
        let clock = TokioClock::new();
        // advance past the 30-year clamp window in deadline.rs
        tokio::time::advance(Duration::from_secs(1_000_000_000)).await;

        let tokio_now = tokio::time::Instant::now();
        let delay = Duration::from_secs(5);
        let target = clock.now().saturating_add(delay);

        assert_eq!(
            clock.to_tokio_instant(target),
            tokio_now.checked_add(delay).unwrap()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn monotonic_clock_deadline_and_has_reached() {
        let clock = TokioClock::new();
        tokio::time::advance(Duration::from_secs(10)).await;

        let target = clock.deadline(Duration::from_secs(5));
        assert_eq!(target, MonotonicTime::from_elapsed(Duration::from_secs(15)));
        assert!(!clock.has_reached(target));

        tokio::time::advance(Duration::from_millis(4_999)).await;
        assert!(!clock.has_reached(target));

        // exact boundary: now == target counts as reached
        tokio::time::advance(Duration::from_millis(1)).await;
        assert!(clock.has_reached(target));

        // past boundary
        tokio::time::advance(Duration::from_secs(1)).await;
        assert!(clock.has_reached(target));

        // saturation
        assert_eq!(
            clock.deadline(Duration::MAX),
            MonotonicTime::from_elapsed(Duration::MAX)
        );
    }
}
