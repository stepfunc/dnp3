// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

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
