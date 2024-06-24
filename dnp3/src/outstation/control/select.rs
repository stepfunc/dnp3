use crate::app::control::CommandStatus;
use crate::app::{Sequence, Timeout};

/// records when a select occurs
#[derive(Copy, Clone)]
pub(crate) struct SelectState {
    /// sequence number of the SELECT
    seq: Sequence,
    /// frame count of the select, makes it easier to ensure that OPERATE directly follows SELECT
    /// without requests in between
    frame_id: u32,
    /// time at which the SELECT occurred
    time: tokio::time::Instant,
    /// the hash of the object headers
    object_hash: u64,
}

impl SelectState {
    pub(crate) fn new(
        seq: Sequence,
        frame_id: u32,
        time: tokio::time::Instant,
        object_hash: u64,
    ) -> Self {
        Self {
            seq,
            frame_id,
            time,
            object_hash,
        }
    }

    pub(crate) fn update_frame_id(&mut self, new_frame_id: u32) {
        self.frame_id = new_frame_id;
    }

    pub(crate) fn match_operate(
        &self,
        timeout: Timeout,
        seq: Sequence,
        frame_id: u32,
        object_hash: u64,
    ) -> Result<(), CommandStatus> {
        let elapsed = tokio::time::Instant::now().checked_duration_since(self.time);

        // check the sequence number
        if self.seq.next() != seq.value() {
            tracing::warn!("received OPERATE with non-consecutive sequence number");
            return Err(CommandStatus::NoSelect);
        }

        // check the frame_id to ensure there was no requests in between the SELECT and OPERATE
        if self.frame_id.wrapping_add(1) != frame_id {
            tracing::warn!("received OPERATE without prior SELECT");
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
