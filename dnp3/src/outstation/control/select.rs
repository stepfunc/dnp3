use crate::app::control::CommandStatus;
use crate::app::{Sequence, Timeout};

/// records when a select occurs
#[derive(Copy, Clone)]
pub(crate) struct SelectState {
    /// sequence number of the SELECT
    seq: Sequence,
    /// hash of the SELECT request, which recognizes its retry
    request_hash: u64,
    /// time at which the SELECT occurred
    time: tokio::time::Instant,
    /// the hash of the object headers
    object_hash: u64,
}

impl SelectState {
    pub(crate) fn new(
        seq: Sequence,
        request_hash: u64,
        time: tokio::time::Instant,
        object_hash: u64,
    ) -> Self {
        Self {
            seq,
            request_hash,
            time,
            object_hash,
        }
    }

    pub(crate) fn seq(&self) -> Sequence {
        self.seq
    }

    pub(crate) fn request_hash(&self) -> u64 {
        self.request_hash
    }

    pub(crate) fn match_operate(
        &self,
        timeout: Timeout,
        seq: Sequence,
        object_hash: u64,
    ) -> Result<(), CommandStatus> {
        let elapsed = tokio::time::Instant::now().checked_duration_since(self.time);

        // check the sequence number
        if self.seq.next() != seq.value() {
            tracing::warn!("received OPERATE with non-consecutive sequence number");
            return Err(CommandStatus::NoSelect);
        }

        // check the object hash
        if self.object_hash != object_hash {
            tracing::warn!("received OPERATE with different header than SELECT");
            return Err(CommandStatus::NoSelect);
        }

        // check the time last
        match elapsed {
            None => {
                tracing::error!("current time is less than time of SELECT, clock error?");
                return Err(CommandStatus::Timeout);
            }
            Some(elapsed) => {
                if elapsed > timeout.into() {
                    tracing::warn!("received valid OPERATE after SELECT timeout");
                    return Err(CommandStatus::Timeout);
                }
            }
        }

        Ok(())
    }
}
