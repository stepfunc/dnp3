use std::time::Duration;

use tokio::time::Instant;

/// Delays longer than this are treated as this long, so a deadline can always be represented
const MAX_DELAY: Duration = Duration::from_secs(946_080_000); // 30 years

/// The instant `delay` after `now`
pub(crate) fn deadline(now: Instant, delay: Duration) -> Instant {
    now.checked_add(delay.min(MAX_DELAY)).unwrap_or(now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_ordinary_delays() {
        let now = Instant::now();
        let delay = Duration::from_secs(5);
        assert_eq!(deadline(now, delay), now.checked_add(delay).unwrap());
    }

    #[test]
    fn clamps_delays_too_long_to_represent() {
        let now = Instant::now();
        assert_eq!(
            deadline(now, Duration::MAX),
            now.checked_add(MAX_DELAY).unwrap()
        );
    }
}
