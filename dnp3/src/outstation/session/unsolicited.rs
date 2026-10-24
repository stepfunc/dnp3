// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::*;

impl OutstationSession {
    pub(super) async fn check_unsolicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        out: &mut Responder<T>,
        database: &mut DatabaseHandle,
    ) -> Result<NextIdleAction, RunError> {
        if self.config.unsolicited.is_disabled() {
            return Ok(NextIdleAction::SleepUntilEvent);
        }

        match self.state.unsolicited {
            UnsolicitedState::NullRequired => {
                // perform NULL unsolicited
                match self
                    .perform_null_unsolicited(io, reader, out, database)
                    .await?
                {
                    ConfirmOutcome::Unconfirmed => {
                        self.end_confirm_wait(database, ConfirmOutcome::Unconfirmed)
                            .await;
                        self.state.unsolicited = UnsolicitedState::NullRequired;
                        Ok(NextIdleAction::NoSleep)
                    }
                    ConfirmOutcome::Confirmed => {
                        self.state.unsolicited = UnsolicitedState::Ready(None);
                        Ok(NextIdleAction::NoSleep)
                    }
                }
            }
            UnsolicitedState::Ready(deadline) => {
                if let Some(deadline) = deadline {
                    if tokio::time::Instant::now() < deadline {
                        return Ok(NextIdleAction::SleepUntil(deadline)); // not ready yet
                    }
                }

                // perform regular unsolicited
                let Some(outcome) = self
                    .maybe_perform_unsolicited(io, reader, out, database)
                    .await?
                else {
                    // there was nothing to send
                    return Ok(NextIdleAction::SleepUntilEvent);
                };
                self.end_confirm_wait(database, outcome).await;
                match outcome {
                    ConfirmOutcome::Unconfirmed => {
                        let retry_at = self.new_unsolicited_retry_deadline();
                        self.state.unsolicited = UnsolicitedState::Ready(Some(retry_at));
                        Ok(NextIdleAction::SleepUntil(retry_at))
                    }
                    ConfirmOutcome::Confirmed => {
                        self.state.unsolicited = UnsolicitedState::Ready(None);
                        Ok(NextIdleAction::NoSleep)
                    }
                }
            }
        }
    }

    pub(super) async fn perform_null_unsolicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        out: &mut Responder<T>,
        database: &mut DatabaseHandle,
    ) -> Result<ConfirmOutcome, RunError> {
        let header = ResponseHeader::new(
            ControlField::unsolicited_response(self.state.unsolicited_seq.increment()),
            ResponseFunction::UnsolicitedResponse,
            Iin::default(),
        );
        let seq = self
            .write_unsolicited(io, &mut out.writer, out.unsol.empty(header), database)
            .await?
            .control
            .seq;
        // For null responses, we want to regenerate a response everytime (see section 5.1.1.1.1 Rule 2)
        self.unsolicited_confirm_wait(database, seq, Some(0), io, reader, out)
            .await
    }

    pub(super) async fn maybe_perform_unsolicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        out: &mut Responder<T>,
        database: &mut DatabaseHandle,
    ) -> Result<Option<ConfirmOutcome>, RunError> {
        if !self.state.enabled_unsolicited_classes.any() {
            return Ok(None);
        }

        let Some(response) = self.write_unsolicited_data(&mut out.unsol, database) else {
            return Ok(None);
        };
        let seq = self
            .write_unsolicited(io, &mut out.writer, response, database)
            .await?
            .control
            .seq;
        let result = self
            .unsolicited_confirm_wait(
                database,
                seq,
                self.config.max_unsolicited_retries,
                io,
                reader,
                out,
            )
            .await?;
        Ok(Some(result))
    }

    pub(super) fn write_unsolicited_data<'a>(
        &mut self,
        unsol: &'a mut ResponseSlot,
        database: &mut DatabaseHandle,
    ) -> Option<Prepared<'a>> {
        let classes = self.state.enabled_unsolicited_classes;
        let (count, staged) = unsol.build(|cursor| Ok(database.write_unsolicited(classes, cursor)));

        if count.unwrap_or_default() == 0 {
            return None;
        }

        let seq = self.state.unsolicited_seq.increment();
        let header = ResponseHeader::new(
            ControlField::unsolicited_response(seq),
            ResponseFunction::UnsolicitedResponse,
            Iin::default(),
        );
        Some(staged.with_header(header))
    }

    pub(super) fn new_unsolicited_retry_deadline(&self) -> tokio::time::Instant {
        deadline(
            tokio::time::Instant::now(),
            self.config.unsolicited_retry_delay,
        )
    }
}
