// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::*;

impl OutstationSession {
    pub(super) async fn sol_confirm_wait<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        out: &mut Responder<T>,
        database: &mut DatabaseHandle,
        series: ResponseSeries,
    ) -> Result<SolicitedWait, RunError> {
        let span = tracing::info_span!("SolConfirmWait", "ecsn" = series.ecsn.value());
        self.run_sol_confirm_wait(io, reader, out, database, series)
            .instrument(span)
            .await
    }

    pub(super) async fn run_sol_confirm_wait<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        out: &mut Responder<T>,
        database: &mut DatabaseHandle,
        mut series: ResponseSeries,
    ) -> Result<SolicitedWait, RunError> {
        self.info.enter_solicited_confirm_wait(series.ecsn);

        loop {
            match self
                .wait_for_sol_confirm(io, reader, out, series.ecsn)
                .await?
            {
                Confirm::Yes(respond_to) => {
                    self.state.broadcast.confirmed(ResponseKind::Solicited);

                    self.end_confirm_wait(database, ConfirmOutcome::Confirmed)
                        .await;

                    if series.fin {
                        // done with response series
                        return Ok(SolicitedWait::Done);
                    }
                    // format the next response in the series
                    series.ecsn.increment();
                    let response = Self::format_read_response(
                        &mut out.sol,
                        database,
                        false,
                        series.ecsn,
                        Iin2::default(),
                    );
                    let header = self
                        .write_solicited(io, &mut out.writer, respond_to, response, database)
                        .await?;
                    match Self::awaited_confirm(header) {
                        None => return Ok(SolicitedWait::Done),
                        Some(next) => {
                            series = next;
                        }
                    }
                }
                Confirm::Timeout => {
                    tracing::warn!("confirm timeout");
                    self.end_confirm_wait(database, ConfirmOutcome::Unconfirmed)
                        .await;
                    return Ok(SolicitedWait::Done);
                }
                Confirm::NewRequest => {
                    tracing::info!("aborting solicited response due to new request");
                    self.end_confirm_wait(database, ConfirmOutcome::Unconfirmed)
                        .await;
                    return Ok(SolicitedWait::RequestPending);
                }
            }
        }
    }

    pub(super) async fn wait_for_sol_confirm<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        out: &mut Responder<T>,
        ecsn: Sequence,
    ) -> Result<Confirm, RunError> {
        let mut deadline = self.new_confirm_deadline();
        loop {
            match self.read_until(io, reader, deadline).await? {
                TimeoutStatus::Yes => {
                    self.info.solicited_confirm_timeout(ecsn);
                    return Ok(Confirm::Timeout);
                }
                // process data
                TimeoutStatus::No => {
                    let mut guard = self.pop_request(reader);
                    match self.expect_sol_confirm(ecsn, &guard) {
                        ConfirmAction::ContinueWait => {
                            // we ignored whatever the request was and logged it elsewhere
                            // just go back to the loop and read another fragment
                        }
                        ConfirmAction::Confirmed(respond_to) => {
                            self.info.solicited_confirm_received(ecsn);
                            return Ok(Confirm::Yes(respond_to));
                        }
                        ConfirmAction::NewRequest => {
                            self.info.solicited_confirm_wait_new_request();
                            // retain the fragment so that it can be processed from the idle state
                            guard.retain();
                            return Ok(Confirm::NewRequest);
                        }
                        ConfirmAction::EchoLastResponse(respond_to) => {
                            self.repeat_solicited(io, respond_to, out).await?;
                            // per the spec, we restart the confirm timer
                            deadline = self.new_confirm_deadline();
                        }
                    }
                }
            }
        }
    }

    pub(super) fn expect_sol_confirm<T: Transport>(
        &mut self,
        ecsn: Sequence,
        guard: &RequestGuard<'_, T>,
    ) -> ConfirmAction {
        match self.receive(guard) {
            Intake::Nothing => ConfirmAction::ContinueWait,
            Intake::UnknownFunction(_, _) => ConfirmAction::NewRequest,
            Intake::Confirm(from, header) => {
                let seq = header.control.seq;
                if header.control.uns {
                    self.info.unexpected_confirm(true, seq);
                    tracing::warn!("ignoring unsolicited confirm with seq: {}", seq.value());
                    ConfirmAction::ContinueWait
                } else if seq == ecsn {
                    ConfirmAction::Confirmed(from)
                } else {
                    self.info.wrong_solicited_confirm_seq(ecsn, seq);
                    tracing::warn!(
                        "ignoring confirm with wrong sequence number: {}",
                        seq.value()
                    );
                    ConfirmAction::ContinueWait
                }
            }
            Intake::Request(received) => match received.fragment {
                FragmentType::RepeatRead(_, _) => {
                    // a request ends the selection; the READ that started this wait already did
                    self.state.select = None;
                    ConfirmAction::EchoLastResponse(received.info.addr)
                }
                // a new request ends the wait, and is answered from idle
                FragmentType::MalformedRequest(_, _)
                | FragmentType::NewRead(_, _)
                | FragmentType::NewNonRead(_, _)
                | FragmentType::RepeatNonRead(_)
                | FragmentType::Broadcast(_) => ConfirmAction::NewRequest,
            },
        }
    }

    /// Wait for the confirm of the unsolicited response just sent with `seq`, sending it again
    /// when the wait times out, up to `max_retries` times (`None` for no limit)
    pub(super) async fn unsolicited_confirm_wait<T: Transport>(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        max_retries: Option<usize>,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        out: &mut Responder<T>,
    ) -> Result<ConfirmOutcome, RunError> {
        // enter unsolicited confirm wait state
        self.info.enter_unsolicited_confirm_wait(seq);

        let mut deadline = self.new_confirm_deadline();

        // IEEE 1815-2012 Table 6-1 rows 22 and 33 would go on to wait for the solicited confirm
        // after the unsolicited response's wait ends. This wait never does: confirming a
        // solicited response from here only clears a broadcast's IIN1.0, while the solicited
        // confirm wait would settle the unsolicited response's events
        let mut wait = UnsolicitedWait {
            seq,
            solicited: None,
            retries: RetryCounter::new(max_retries),
        };

        loop {
            match self
                .wait_for_unsolicited_confirm(&mut wait, deadline, io, reader, out, database)
                .instrument(tracing::info_span!("UnsolConfirmWait", "seq" = seq.value()))
                .await?
            {
                UnsolicitedWaitResult::ReadNext => {
                    // just go to next iteration without changing the deadline
                }
                UnsolicitedWaitResult::Complete(result) => return Ok(result),
                UnsolicitedWaitResult::Timeout => {
                    let mut retry = wait.retries.decrement();

                    // If a deferred read is pending, we want to exit
                    if self.state.deferred_read.is_set() {
                        retry = false;
                    }

                    self.info.unsolicited_confirm_timeout(seq, retry);

                    if !retry {
                        return Ok(ConfirmOutcome::Unconfirmed);
                    }

                    // perform a retry
                    self.repeat_unsolicited(io, out).await?;

                    // update the deadline
                    deadline = self.new_confirm_deadline();
                }
            }
        }
    }

    pub(super) async fn wait_for_unsolicited_confirm<T: Transport>(
        &mut self,
        wait: &mut UnsolicitedWait,
        deadline: tokio::time::Instant,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        out: &mut Responder<T>,
        database: &mut DatabaseHandle,
    ) -> Result<UnsolicitedWaitResult, RunError> {
        if let TimeoutStatus::Yes = self.read_until(io, reader, deadline).await? {
            return Ok(UnsolicitedWaitResult::Timeout);
        }

        let guard = self.pop_request(reader);
        let received = match self.receive(&guard) {
            Intake::Nothing => return Ok(UnsolicitedWaitResult::ReadNext),
            Intake::UnknownFunction(from, seq) => {
                // like a new request, it ends the wait for a solicited confirm, as it ends the
                // solicited confirm wait. Its IIN2.0 answer isn't the response to a valid request,
                // so the outstation doesn't wait for that answer's confirm
                wait.solicited = None;
                self.answer_unknown_function(io, &mut out.writer, from, seq, database)
                    .await?;
                return Ok(UnsolicitedWaitResult::ReadNext);
            }
            Intake::Confirm(_, header) => {
                return Ok(self.confirm_in_unsolicited_wait(wait, header));
            }
            Intake::Request(received) => received,
        };

        let selection = self.take_selection(&received.fragment);
        let function = received.request.header.function;
        let direct = matches!(received.fragment, FragmentType::NewNonRead(..));

        // Table 6-1 rows 24-29: a request ends the wait for a solicited confirm, and an answer that
        // asks for one starts it again
        let accepted = self
            .accept(io, out, database, received, selection, Reads::Defer)
            .await?;
        wait.solicited = accepted.confirm.map(|series| ExpectedConfirm {
            ecsn: series.ecsn,
            deadline: self.new_confirm_deadline(),
        });

        // any DISABLE_UNSOLICITED processed, directly or as a broadcast, whatever classes it names
        if function == FunctionCode::DisableUnsolicited
            && (direct || accepted.broadcast == Some(BroadcastAction::Processed))
        {
            wait.retries.stop();
        }

        Ok(UnsolicitedWaitResult::ReadNext)
    }

    pub(super) fn confirm_in_unsolicited_wait(
        &mut self,
        wait: &mut UnsolicitedWait,
        header: RequestHeader,
    ) -> UnsolicitedWaitResult {
        let seq = header.control.seq;
        if header.control.uns {
            if seq == wait.seq {
                self.state.broadcast.confirmed(ResponseKind::Unsolicited);
                self.info.unsolicited_confirmed(seq);
                return UnsolicitedWaitResult::Complete(ConfirmOutcome::Confirmed);
            }
            tracing::warn!(
                "ignoring unsolicited confirm with wrong sequence number ({})",
                seq.value()
            );
        } else {
            // Table 6-1 rows 20, 21 and 30: only the expected confirm, before its timeout
            let now = tokio::time::Instant::now();
            wait.solicited.take_if(|expected| now >= expected.deadline);
            if wait
                .solicited
                .take_if(|expected| expected.ecsn == seq)
                .is_some()
            {
                self.state.broadcast.confirmed(ResponseKind::Solicited);
            } else {
                tracing::warn!("ignoring solicited confirm with seq: {}", seq.value());
            }
        }
        UnsolicitedWaitResult::ReadNext
    }

    /// Settle the events written into a response once its confirm wait ends: delete them if the
    /// master confirmed it, otherwise return them to the unselected state to be reported again
    pub(super) async fn end_confirm_wait(
        &mut self,
        database: &mut DatabaseHandle,
        outcome: ConfirmOutcome,
    ) {
        match outcome {
            ConfirmOutcome::Confirmed => {
                database
                    .clear_written_events(self.application.as_mut())
                    .await
            }
            ConfirmOutcome::Unconfirmed => database.reset(),
        }
    }

    pub(super) fn new_confirm_deadline(&self) -> tokio::time::Instant {
        self.config.confirm_timeout.deadline_from_now()
    }
}
