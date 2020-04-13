use std::time::Duration;
use tokio::time::Instant;

pub(crate) struct Timeout {
    pub(crate) value: Instant,
}

impl Timeout {
    pub(crate) fn from_now(duration: Duration) -> Self {
        Self {
            value: Instant::now() + duration,
        }
    }

    pub(crate) fn extend(&mut self, duration: Duration) {
        self.value = Instant::now() + duration
    }
}
