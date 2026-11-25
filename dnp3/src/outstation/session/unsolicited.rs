// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::context::SessionContext;
use super::response::ResponseSlot;
use super::state::{RetryCounter, UnsolicitedState, UnsolicitedWaitKind};
use crate::app::{ControlField, Iin, ResponseFunction, ResponseHeader, Sequence};
use crate::outstation::database::DatabaseHandle;
use crate::util::clock::{MonotonicClock, MonotonicTime};

impl<C: MonotonicClock> SessionContext<C> {
    pub(super) fn new_unsolicited_retry_deadline(&self) -> MonotonicTime {
        self.clock.deadline(self.config.unsolicited_retry_delay)
    }

    /// Try to publish a NULL or event unsolicited response into `unsol`.
    ///
    /// When a retry delay has not yet elapsed, leaves `retry_at` in `self.state.unsolicited` so
    /// `Waiting::deadline` reports it.
    pub(super) fn stage_unsolicited(
        &mut self,
        unsol: &mut ResponseSlot,
        db: &mut DatabaseHandle,
    ) -> Option<(Sequence, UnsolicitedWaitKind)> {
        match self.state.unsolicited {
            UnsolicitedState::Disabled => None,
            UnsolicitedState::NullRequired => Some(self.stage_null_unsolicited(unsol, db)),
            UnsolicitedState::Ready { retry_at } => {
                if let Some(deadline) = retry_at {
                    if !self.clock.has_reached(deadline) {
                        return None;
                    }
                    // Retry delay has elapsed; clear it so we don't re-enter with a stale deadline.
                    self.state.unsolicited = UnsolicitedState::Ready { retry_at: None };
                }
                self.stage_unsolicited_events(unsol, db)
            }
        }
    }

    fn next_unsolicited_header(&mut self) -> ResponseHeader {
        let seq = self.state.unsolicited_seq.increment();
        ResponseHeader::new(
            ControlField::unsolicited_response(seq),
            ResponseFunction::UnsolicitedResponse,
            Iin::default(),
        )
    }

    fn stage_null_unsolicited(
        &mut self,
        unsol: &mut ResponseSlot,
        db: &mut DatabaseHandle,
    ) -> (Sequence, UnsolicitedWaitKind) {
        let header = self.next_unsolicited_header();
        let seq = self.publish_unsolicited(unsol.empty(header), db);
        (seq, UnsolicitedWaitKind::Null)
    }

    fn stage_unsolicited_events(
        &mut self,
        unsol: &mut ResponseSlot,
        db: &mut DatabaseHandle,
    ) -> Option<(Sequence, UnsolicitedWaitKind)> {
        if !self.state.enabled_unsolicited_classes.any() {
            return None;
        }

        let classes = self.state.enabled_unsolicited_classes;
        let (count, staged) = unsol.build(|cursor| Ok(db.write_unsolicited(classes, cursor)));

        if count.unwrap_or_default() == 0 {
            return None;
        }

        let header = self.next_unsolicited_header();
        let seq = self.publish_unsolicited(staged.with_header(header), db);
        Some((
            seq,
            UnsolicitedWaitKind::Events {
                retries: RetryCounter::new(self.config.max_unsolicited_retries),
            },
        ))
    }
}
