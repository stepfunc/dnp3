// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::context::SessionContext;
use super::core::Outbound;
use super::response::{awaited_confirm, Slots};
use super::state::{
    ConfirmWaitInfo, ExpectedConfirm, FragmentType, Intake, ResponseKind, ResponseSeries,
    SolConfirmWait, UnsolConfirmWait, UnsolicitedState, UnsolicitedWaitKind,
};
use crate::app::{FunctionCode, ResponseHeader, Sequence};
use crate::outstation::database::DatabaseHandle;
use crate::outstation::traits::BroadcastAction;
use crate::transport::FragmentAddr;
use crate::util::clock::{MonotonicClock, MonotonicTime};

/// Result of handling an accepted request in [`SessionContext::accept_request`].
pub(super) enum Answer {
    /// A solicited response was published in `sol` and is ready to send.
    Respond(SolTx),
    /// Valid request with no response (e.g., `*_NO_ACK`, or repeat of one).
    Silent,
    /// A broadcast request was processed with no response.
    Broadcast(BroadcastAction),
}

/// A solicited response published in `sol`, ready to transmit to `to`.
///
/// Fields are private so `awaits` is always derived from the published response header.
#[derive(Copy, Clone, Debug)]
pub(super) struct SolTx {
    to: FragmentAddr,
    awaits: Option<ResponseSeries>,
}

impl SolTx {
    pub(super) fn new(to: FragmentAddr, header: ResponseHeader) -> Self {
        Self {
            to,
            awaits: awaited_confirm(header),
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum IdleTx {
    Solicited(SolTx),
    Transient(FragmentAddr),
    Unsolicited {
        seq: Sequence,
        kind: UnsolicitedWaitKind,
    },
    LinkStatus,
}

#[derive(Copy, Clone, Debug)]
enum UnsolTx {
    Solicited(SolTx),
    Transient(FragmentAddr),
    Retry,
}

/// Phase with no transmission staged: waiting on input, timers, or database updates.
pub(super) struct Waiting(WaitingState);

enum WaitingState {
    Idle,
    SolConfirmWait(SolConfirmWait),
    UnsolConfirmWait(UnsolConfirmWait),
}

/// Phase with a transmission staged, pairing it with the exact phase it was staged from.
#[derive(Copy, Clone, Debug)]
pub(super) struct StagedTx(StagedState);

#[derive(Copy, Clone, Debug)]
enum StagedState {
    Idle(IdleTx),
    SolWait(SolConfirmWait, FragmentAddr),
    UnsolWait(UnsolConfirmWait, UnsolTx),
}

/// Move-only outstation application-layer phase: either waiting or carrying a staged transmission.
///
/// All phase transitions live in this module: each transition method consumes `self` by value and
/// returns the next [`Phase`] or [`Waiting`].
pub(super) enum Phase {
    Waiting(Waiting),
    Staged(StagedTx),
}

impl Waiting {
    /// Active confirm-wait tracing span metadata, if waiting for a confirm.
    pub(super) fn confirm_wait_info(&self) -> Option<ConfirmWaitInfo> {
        match self.0 {
            WaitingState::Idle => None,
            WaitingState::SolConfirmWait(wait) => {
                Some(ConfirmWaitInfo::Solicited(wait.series.ecsn))
            }
            WaitingState::UnsolConfirmWait(wait) => Some(ConfirmWaitInfo::Unsolicited(wait.seq)),
        }
    }

    /// Whether new events added to the database can immediately trigger an unsolicited response.
    pub(super) fn watches_database<C: MonotonicClock>(&self, ctx: &SessionContext<C>) -> bool {
        matches!(self.0, WaitingState::Idle)
            && matches!(
                ctx.state.unsolicited,
                UnsolicitedState::Ready { retry_at: None }
            )
            && ctx.state.enabled_unsolicited_classes.any()
    }

    /// Next timer deadline relevant while waiting.
    pub(super) fn deadline<C: MonotonicClock>(
        &self,
        ctx: &SessionContext<C>,
    ) -> Option<MonotonicTime> {
        match self.0 {
            WaitingState::SolConfirmWait(wait) => Some(wait.deadline),
            WaitingState::UnsolConfirmWait(wait) => Some(wait.deadline),
            WaitingState::Idle => {
                let unsol = match ctx.state.unsolicited {
                    UnsolicitedState::Ready { retry_at } => retry_at,
                    UnsolicitedState::Disabled | UnsolicitedState::NullRequired => None,
                };
                match (unsol, ctx.keep_alive.next_deadline()) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (a, b) => a.or(b),
                }
            }
        }
    }
}

impl StagedTx {
    /// Resolve the staged transmission into its outbound payload from `slots`.
    pub(super) fn outbound<'a>(
        &self,
        slots: &'a Slots,
        default_dest: FragmentAddr,
    ) -> Option<Outbound<'a>> {
        let (destination, bytes) = match self.0 {
            StagedState::Idle(IdleTx::Solicited(sol))
            | StagedState::UnsolWait(_, UnsolTx::Solicited(sol)) => (sol.to, slots.sol.ready()?),
            StagedState::SolWait(_, to) => (to, slots.sol.ready()?),
            StagedState::Idle(IdleTx::Unsolicited { .. })
            | StagedState::UnsolWait(_, UnsolTx::Retry) => (default_dest, slots.unsol.ready()?),
            StagedState::Idle(IdleTx::Transient(to))
            | StagedState::UnsolWait(_, UnsolTx::Transient(to)) => (to, slots.transient.as_slice()),
            StagedState::Idle(IdleTx::LinkStatus) => {
                return Some(Outbound::LinkStatusRequest {
                    destination: default_dest,
                });
            }
        };
        Some(Outbound::Fragment { destination, bytes })
    }

    /// Drop the staged transmission while keeping the active wait phase.
    pub(super) fn unstage(self) -> Waiting {
        match self.0 {
            StagedState::Idle(_) => Waiting(WaitingState::Idle),
            StagedState::SolWait(wait, _) => Waiting(WaitingState::SolConfirmWait(wait)),
            StagedState::UnsolWait(wait, _) => Waiting(WaitingState::UnsolConfirmWait(wait)),
        }
    }

    /// Active confirm-wait tracing span metadata, if staged from a confirm wait.
    pub(super) fn confirm_wait_info(&self) -> Option<ConfirmWaitInfo> {
        self.unstage().confirm_wait_info()
    }

    /// Commit the post-send transition once this transmission finishes writing.
    pub(super) fn on_sent<C: MonotonicClock>(self, ctx: &mut SessionContext<C>) -> Waiting {
        let confirm_deadline = || ctx.clock.deadline(ctx.config.confirm_timeout.into());
        match self.0 {
            StagedState::Idle(IdleTx::Solicited(SolTx {
                awaits: Some(series),
                ..
            })) => {
                ctx.info.enter_solicited_confirm_wait(series.ecsn);
                Waiting(WaitingState::SolConfirmWait(SolConfirmWait {
                    series,
                    deadline: confirm_deadline(),
                }))
            }
            StagedState::Idle(IdleTx::Unsolicited { seq, kind }) => {
                ctx.info.enter_unsolicited_confirm_wait(seq);
                Waiting(WaitingState::UnsolConfirmWait(UnsolConfirmWait {
                    seq,
                    deadline: confirm_deadline(),
                    solicited: None,
                    kind,
                }))
            }
            StagedState::Idle(IdleTx::LinkStatus) => {
                ctx.keep_alive.on_activity(ctx.clock.now());
                Waiting(WaitingState::Idle)
            }
            StagedState::Idle(
                IdleTx::Solicited(SolTx { awaits: None, .. }) | IdleTx::Transient(_),
            ) => Waiting(WaitingState::Idle),

            StagedState::SolWait(mut wait, _) => {
                wait.deadline = confirm_deadline();
                Waiting(WaitingState::SolConfirmWait(wait))
            }

            StagedState::UnsolWait(mut wait, UnsolTx::Solicited(sol)) => {
                wait.solicited = sol.awaits.map(|series| ExpectedConfirm {
                    ecsn: series.ecsn,
                    deadline: confirm_deadline(),
                });
                Waiting(WaitingState::UnsolConfirmWait(wait))
            }
            StagedState::UnsolWait(wait, UnsolTx::Transient(_)) => {
                Waiting(WaitingState::UnsolConfirmWait(wait))
            }
            StagedState::UnsolWait(mut wait, UnsolTx::Retry) => {
                wait.deadline = confirm_deadline();
                Waiting(WaitingState::UnsolConfirmWait(wait))
            }
        }
    }
}

impl Phase {
    pub(super) fn idle() -> Self {
        Self::Waiting(Waiting(WaitingState::Idle))
    }

    fn sol_wait(wait: SolConfirmWait) -> Self {
        Self::Waiting(Waiting(WaitingState::SolConfirmWait(wait)))
    }

    fn unsol_wait(wait: UnsolConfirmWait) -> Self {
        Self::Waiting(Waiting(WaitingState::UnsolConfirmWait(wait)))
    }

    fn stage_idle(tx: IdleTx) -> Self {
        Self::Staged(StagedTx(StagedState::Idle(tx)))
    }

    fn stage_sol(wait: SolConfirmWait, to: FragmentAddr) -> Self {
        Self::Staged(StagedTx(StagedState::SolWait(wait, to)))
    }

    fn stage_unsol(wait: UnsolConfirmWait, tx: UnsolTx) -> Self {
        Self::Staged(StagedTx(StagedState::UnsolWait(wait, tx)))
    }

    pub(super) fn confirm_wait_info(&self) -> Option<ConfirmWaitInfo> {
        match self {
            Self::Waiting(waiting) => waiting.confirm_wait_info(),
            Self::Staged(staged) => staged.confirm_wait_info(),
        }
    }

    /// Drive timers and outbound staging, leaving any already-staged transmission intact.
    pub(super) fn on_poll<C: MonotonicClock>(
        self,
        slots: &mut Slots,
        ctx: &mut SessionContext<C>,
        db: &mut DatabaseHandle,
    ) -> Self {
        let waiting = match self {
            Self::Staged(staged) => return Self::Staged(staged),
            Self::Waiting(waiting) => waiting,
        };

        match waiting.0 {
            WaitingState::SolConfirmWait(wait) => {
                if !ctx.clock.has_reached(wait.deadline) {
                    return Self::sol_wait(wait);
                }
                ctx.info.solicited_confirm_timeout(wait.series.ecsn);
                tracing::warn!("confirm timeout");
                db.reset();
                Self::idle_poll(slots, ctx, db)
            }

            WaitingState::UnsolConfirmWait(wait) => {
                if !ctx.clock.has_reached(wait.deadline) {
                    return Self::unsol_wait(wait);
                }
                Self::unsol_confirm_timeout(wait, slots, ctx, db)
            }

            WaitingState::Idle => Self::idle_poll(slots, ctx, db),
        }
    }

    /// Process one incoming fragment or link-layer message, dropping any previously staged tx.
    pub(super) async fn on_input<C: MonotonicClock>(
        self,
        slots: &mut Slots,
        ctx: &mut SessionContext<C>,
        intake: Intake<'_>,
        db: &mut DatabaseHandle,
    ) -> Self {
        let waiting = match self {
            Self::Waiting(waiting) => waiting,
            Self::Staged(staged) => staged.unstage(),
        };
        match waiting.0 {
            WaitingState::Idle => Self::idle_input(slots, ctx, intake, db).await,
            WaitingState::SolConfirmWait(wait) => {
                Self::sol_wait_input(wait, slots, ctx, intake, db).await
            }
            WaitingState::UnsolConfirmWait(wait) => {
                Self::unsol_wait_input(wait, slots, ctx, intake, db).await
            }
        }
    }

    fn idle_poll<C: MonotonicClock>(
        slots: &mut Slots,
        ctx: &mut SessionContext<C>,
        db: &mut DatabaseHandle,
    ) -> Self {
        // Priority order in Idle: deferred READ -> keep-alive -> unsolicited.
        // Keep-alive runs before unsolicited so an unconfirmed NULL unsolicited loop cannot starve
        // link status requests between confirm waits.
        if let Some(sol) = ctx.stage_deferred_read(&mut slots.sol, db) {
            return Self::stage_idle(IdleTx::Solicited(sol));
        }

        if ctx.keep_alive.is_due(ctx.clock.now()) {
            return Self::stage_idle(IdleTx::LinkStatus);
        }

        if let Some((seq, kind)) = ctx.stage_unsolicited(&mut slots.unsol, db) {
            return Self::stage_idle(IdleTx::Unsolicited { seq, kind });
        }

        Self::idle()
    }

    fn unsol_confirm_timeout<C: MonotonicClock>(
        wait: UnsolConfirmWait,
        slots: &mut Slots,
        ctx: &mut SessionContext<C>,
        db: &mut DatabaseHandle,
    ) -> Self {
        match wait.kind {
            UnsolicitedWaitKind::Null => {
                ctx.info.unsolicited_confirm_timeout(wait.seq, false);
                db.reset();
                ctx.state.unsolicited = UnsolicitedState::NullRequired;
                Self::idle_poll(slots, ctx, db)
            }
            UnsolicitedWaitKind::Events { mut retries } => {
                let retry = !ctx.state.deferred_read.is_set() && retries.decrement();
                ctx.info.unsolicited_confirm_timeout(wait.seq, retry);
                if retry {
                    let wait = UnsolConfirmWait {
                        kind: UnsolicitedWaitKind::Events { retries },
                        ..wait
                    };
                    return Self::stage_unsol(wait, UnsolTx::Retry);
                }

                db.reset();
                ctx.state.unsolicited = UnsolicitedState::Ready {
                    retry_at: Some(ctx.new_unsolicited_retry_deadline()),
                };
                Self::idle_poll(slots, ctx, db)
            }
        }
    }

    async fn idle_input<C: MonotonicClock>(
        slots: &mut Slots,
        ctx: &mut SessionContext<C>,
        intake: Intake<'_>,
        db: &mut DatabaseHandle,
    ) -> Self {
        match intake {
            Intake::UnknownFunction(from, seq) => {
                // Unknown function is not a valid request, so `process_request_from_idle` is not
                // called.
                ctx.stage_unknown_function(&mut slots.transient, seq, db);
                Self::stage_idle(IdleTx::Transient(from))
            }
            Intake::Confirm(_, header) => {
                ctx.info.process_request_from_idle(header);
                let kind = if header.control.uns {
                    "unsolicited"
                } else {
                    "solicited"
                };
                tracing::warn!(
                    "ignoring {kind} CONFIRM from idle state with seq: {}",
                    header.control.seq.value()
                );
                Self::idle()
            }
            Intake::Request(received) => {
                ctx.info.process_request_from_idle(received.header);
                match ctx.accept_request(&mut slots.sol, received, db).await {
                    Answer::Respond(sol) => Self::stage_idle(IdleTx::Solicited(sol)),
                    Answer::Silent | Answer::Broadcast(_) => Self::idle(),
                }
            }
        }
    }

    async fn sol_wait_input<C: MonotonicClock>(
        wait: SolConfirmWait,
        slots: &mut Slots,
        ctx: &mut SessionContext<C>,
        intake: Intake<'_>,
        db: &mut DatabaseHandle,
    ) -> Self {
        match intake {
            Intake::Confirm(from, header) => {
                let seq = header.control.seq;
                if header.control.uns {
                    ctx.info.unexpected_confirm(true, seq);
                    tracing::warn!("ignoring unsolicited confirm with seq: {}", seq.value());
                    return Self::sol_wait(wait);
                }
                if seq != wait.series.ecsn {
                    ctx.info.wrong_solicited_confirm_seq(wait.series.ecsn, seq);
                    tracing::warn!(
                        "ignoring confirm with wrong sequence number: {}",
                        seq.value()
                    );
                    return Self::sol_wait(wait);
                }

                ctx.info.solicited_confirm_received(wait.series.ecsn);
                ctx.state.broadcast.confirmed(ResponseKind::Solicited);
                ctx.clear_written_events(db).await;

                if wait.series.fin {
                    return Self::idle();
                }

                // Format the next response in the multi-fragment series.
                let mut next_seq = wait.series.ecsn;
                next_seq.increment();
                let sol = ctx.next_read_fragment(&mut slots.sol, from, next_seq, db);
                match sol.awaits {
                    Some(series) => Self::stage_sol(
                        SolConfirmWait {
                            series,
                            deadline: wait.deadline,
                        },
                        sol.to,
                    ),
                    None => Self::stage_idle(IdleTx::Solicited(sol)),
                }
            }
            Intake::Request(received)
                if matches!(received.fragment, FragmentType::RepeatRead(_, _)) =>
            {
                Self::stage_sol(wait, received.info.addr)
            }
            intake @ (Intake::UnknownFunction(..) | Intake::Request(_)) => {
                ctx.info.solicited_confirm_wait_new_request();
                tracing::info!("aborting solicited response due to new request");
                db.reset();
                Self::idle_input(slots, ctx, intake, db).await
            }
        }
    }

    async fn unsol_wait_input<C: MonotonicClock>(
        mut wait: UnsolConfirmWait,
        slots: &mut Slots,
        ctx: &mut SessionContext<C>,
        intake: Intake<'_>,
        db: &mut DatabaseHandle,
    ) -> Self {
        match intake {
            Intake::UnknownFunction(from, seq) => {
                // Like a new request, it ends the wait for a solicited confirm. Its IIN2.0 answer
                // isn't the response to a valid request, so the outstation doesn't wait for its
                // confirm.
                wait.solicited = None;
                ctx.stage_unknown_function(&mut slots.transient, seq, db);
                Self::stage_unsol(wait, UnsolTx::Transient(from))
            }
            Intake::Confirm(_, header) => {
                let seq = header.control.seq;
                if header.control.uns {
                    if seq == wait.seq {
                        ctx.state.broadcast.confirmed(ResponseKind::Unsolicited);
                        ctx.info.unsolicited_confirmed(seq);
                        if matches!(wait.kind, UnsolicitedWaitKind::Events { .. }) {
                            ctx.clear_written_events(db).await;
                        }
                        ctx.state.unsolicited = UnsolicitedState::Ready { retry_at: None };
                        return Self::idle();
                    }
                    tracing::warn!(
                        "ignoring unsolicited confirm with wrong sequence number ({})",
                        seq.value()
                    );
                } else {
                    // IEEE 1815-2012 Table 6-1 rows 20, 21 and 30: only the expected confirm,
                    // before its timeout.
                    wait.solicited
                        .take_if(|expected| ctx.clock.has_reached(expected.deadline));
                    if wait
                        .solicited
                        .take_if(|expected| expected.ecsn == seq)
                        .is_some()
                    {
                        ctx.state.broadcast.confirmed(ResponseKind::Solicited);
                    } else {
                        tracing::warn!("ignoring solicited confirm with seq: {}", seq.value());
                    }
                }
                Self::unsol_wait(wait)
            }
            Intake::Request(received) => {
                let function = received.header.function;
                let direct = matches!(received.fragment, FragmentType::NewNonRead { .. });

                // Table 6-1 rows 24-29: a request ends the wait for a solicited confirm; if the
                // answer asks for one, `on_sent` arms it again.
                wait.solicited = None;

                if let FragmentType::NewRead(hash, objects)
                | FragmentType::RepeatRead(hash, objects) = received.fragment
                {
                    ctx.defer_read(hash, received.header.control.seq, received.info, objects);
                    return Self::unsol_wait(wait);
                }

                let answer = ctx.accept_request(&mut slots.sol, received, db).await;

                // TB2015-002a rule 15: a direct or processed broadcast DISABLE_UNSOLICITED stops
                // retries for the active unsolicited wait.
                let stop_retries = function == FunctionCode::DisableUnsolicited
                    && (direct || matches!(answer, Answer::Broadcast(BroadcastAction::Processed)));
                if stop_retries {
                    wait.kind.stop_retries();
                }

                match answer {
                    Answer::Respond(sol) => Self::stage_unsol(wait, UnsolTx::Solicited(sol)),
                    Answer::Silent | Answer::Broadcast(_) => Self::unsol_wait(wait),
                }
            }
        }
    }
}
