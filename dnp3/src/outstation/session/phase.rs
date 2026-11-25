// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::context::SessionContext;
use super::core::Outbound;
use super::response::Slots;
use super::state::{
    ConfirmWaitInfo, ExpectedConfirm, FragmentType, Intake, ResponseKind, ResponseSeries,
    SolConfirmWait, UnsolConfirmWait, UnsolicitedState, UnsolicitedWaitKind,
};
use crate::app::{FunctionCode, ResponseHeader, Sequence};
use crate::decode::AppDecodeLevel;
use crate::outstation::traits::BroadcastAction;
use crate::transport::{FragmentAddr, TransportData};
use crate::util::clock::{MonotonicClock, MonotonicTime};

/// Encapsulated outstation application-layer phase state machine.
///
/// All phase reads and transitions live in this module.
pub(super) struct Phase {
    state: State,
}

impl Phase {
    pub(super) fn idle() -> Self {
        Self {
            state: State::Waiting(Waiting::Idle),
        }
    }

    pub(super) fn reset(&mut self) {
        self.state = State::Waiting(Waiting::Idle);
    }

    pub(super) fn confirm_wait_info(&self) -> Option<ConfirmWaitInfo> {
        match &self.state {
            State::Waiting(waiting) => waiting.confirm_wait_info(),
            State::Staged(staged) => staged.confirm_wait_info(),
            State::InFlight => None,
        }
    }

    /// Drive timers and outbound staging.
    ///
    /// Leaves `self` in [`State::InFlight`] when returning [`PollOutcome::Transmit`] until
    /// [`Self::sent`] commits the post-send phase or [`Self::reset`] clears the session.
    pub(super) fn poll<'a, C: MonotonicClock>(
        &mut self,
        slots: &'a mut Slots,
        ctx: &mut SessionContext<C>,
    ) -> PollOutcome<'a> {
        let waiting = match self.begin(slots, ctx).on_poll(slots, ctx) {
            Active::Waiting(waiting) => waiting,
            Active::Staged(staged) => match staged.outbound(slots, ctx.destination) {
                Some(outbound) => return PollOutcome::Transmit { outbound, staged },
                None => {
                    tracing::error!("staged response missing from slot");
                    staged.unstage()
                }
            },
        };
        let outcome = PollOutcome::Wait(waiting.deadline(ctx));
        self.state = State::Waiting(waiting);
        outcome
    }

    /// Process one incoming fragment or link-layer message, leaving `self` in [`State::InFlight`]
    /// while awaiting callbacks.
    pub(super) async fn input<C: MonotonicClock>(
        &mut self,
        slots: &mut Slots,
        ctx: &mut SessionContext<C>,
        input: TransportData<'_>,
        app_decode_level: AppDecodeLevel,
    ) {
        let active = self.begin(slots, ctx);
        let next = match ctx.receive(input, app_decode_level) {
            Some(intake) => active.on_input(slots, ctx, intake).await,
            None => active,
        };
        self.state = next.into();
    }

    /// Commit the post-send transition once a transmission finishes writing.
    pub(super) fn sent<C: MonotonicClock>(
        &mut self,
        staged: StagedTx,
        ctx: &mut SessionContext<C>,
    ) {
        self.state = State::Waiting(staged.on_sent(ctx));
    }

    /// Take the active phase out, leaving [`State::InFlight`] in place. If the previous operation
    /// was dropped while in flight, reset per-connection state and the database before proceeding.
    fn begin<C: MonotonicClock>(
        &mut self,
        slots: &mut Slots,
        ctx: &mut SessionContext<C>,
    ) -> Active {
        match std::mem::replace(&mut self.state, State::InFlight) {
            State::Waiting(waiting) => Active::Waiting(waiting),
            State::Staged(staged) => Active::Staged(staged),
            State::InFlight => {
                tracing::warn!("recovering outstation core after abandoned operation");
                slots.clear();
                ctx.reset();
                Active::Waiting(Waiting::Idle)
            }
        }
    }
}

/// Outcome of [`Phase::poll`] for [`super::core::OutstationCore`] to wrap into [`super::core::Output`].
pub(super) enum PollOutcome<'a> {
    Wait(Option<MonotonicTime>),
    Transmit {
        outbound: Outbound<'a>,
        staged: StagedTx,
    },
}

/// Move-only staged transmission pairing an outbound send with the phase it was staged from.
///
/// Moves into [`super::core::Transmit`] while a write is underway and is consumed by
/// [`Phase::sent`] once the write completes.
#[derive(Debug)]
pub(super) struct StagedTx {
    state: StagedState,
}

impl StagedTx {
    /// Resolve the staged transmission into its outbound payload from `slots`.
    fn outbound<'a>(&self, slots: &'a Slots, default_dest: FragmentAddr) -> Option<Outbound<'a>> {
        let (destination, bytes) = match self.state {
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
    fn unstage(self) -> Waiting {
        self.state.unstage()
    }

    fn confirm_wait_info(&self) -> Option<ConfirmWaitInfo> {
        self.state.confirm_wait_info()
    }

    /// Commit the post-send transition once this transmission finishes writing.
    fn on_sent<C: MonotonicClock>(self, ctx: &mut SessionContext<C>) -> Waiting {
        let confirm_deadline = || ctx.clock.deadline(ctx.config.confirm_timeout.into());
        match self.state {
            StagedState::Idle(IdleTx::Solicited(SolTx {
                awaits: Some(series),
                ..
            })) => {
                ctx.info.enter_solicited_confirm_wait(series.ecsn);
                Waiting::SolConfirmWait(SolConfirmWait {
                    series,
                    deadline: confirm_deadline(),
                })
            }
            StagedState::Idle(IdleTx::Unsolicited { seq, kind }) => {
                ctx.info.enter_unsolicited_confirm_wait(seq);
                Waiting::UnsolConfirmWait(UnsolConfirmWait {
                    seq,
                    deadline: confirm_deadline(),
                    solicited: None,
                    kind,
                })
            }
            StagedState::Idle(IdleTx::LinkStatus) => {
                ctx.keep_alive.on_activity(ctx.clock.now());
                Waiting::Idle
            }
            StagedState::Idle(
                IdleTx::Solicited(SolTx { awaits: None, .. }) | IdleTx::Transient(_),
            ) => Waiting::Idle,

            StagedState::SolWait(mut wait, _) => {
                wait.deadline = confirm_deadline();
                Waiting::SolConfirmWait(wait)
            }

            StagedState::UnsolWait(mut wait, UnsolTx::Solicited(sol)) => {
                wait.solicited = sol.awaits.map(|series| ExpectedConfirm {
                    ecsn: series.ecsn,
                    deadline: confirm_deadline(),
                });
                Waiting::UnsolConfirmWait(wait)
            }
            StagedState::UnsolWait(wait, UnsolTx::Transient(_)) => Waiting::UnsolConfirmWait(wait),
            StagedState::UnsolWait(mut wait, UnsolTx::Retry) => {
                wait.deadline = confirm_deadline();
                Waiting::UnsolConfirmWait(wait)
            }
        }
    }
}

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
            awaits: ResponseSeries::awaited_by(header),
        }
    }
}

/// Active (non-in-flight) protocol phase.
enum Active {
    Waiting(Waiting),
    Staged(StagedTx),
}

impl Active {
    /// Drive timers and outbound staging, leaving any already-staged transmission intact.
    fn on_poll<C: MonotonicClock>(self, slots: &mut Slots, ctx: &mut SessionContext<C>) -> Self {
        let waiting = match self {
            Self::Staged(staged) => return Self::Staged(staged),
            Self::Waiting(waiting) => waiting,
        };

        match waiting {
            Waiting::SolConfirmWait(wait) => {
                if !ctx.clock.has_reached(wait.deadline) {
                    return Self::sol_wait(wait);
                }
                ctx.info.solicited_confirm_timeout(wait.series.ecsn);
                tracing::warn!("confirm timeout");
                ctx.db.reset();
                Self::idle_poll(slots, ctx)
            }

            Waiting::UnsolConfirmWait(wait) => {
                if !ctx.clock.has_reached(wait.deadline) {
                    return Self::unsol_wait(wait);
                }
                Self::unsol_confirm_timeout(wait, slots, ctx)
            }

            Waiting::Idle => Self::idle_poll(slots, ctx),
        }
    }

    /// Process one incoming fragment or link-layer message, dropping any previously staged tx.
    async fn on_input<C: MonotonicClock>(
        self,
        slots: &mut Slots,
        ctx: &mut SessionContext<C>,
        intake: Intake<'_>,
    ) -> Self {
        let waiting = match self {
            Self::Waiting(waiting) => waiting,
            Self::Staged(staged) => staged.unstage(),
        };
        match waiting {
            Waiting::Idle => Self::idle_input(slots, ctx, intake).await,
            Waiting::SolConfirmWait(wait) => Self::sol_wait_input(wait, slots, ctx, intake).await,
            Waiting::UnsolConfirmWait(wait) => {
                Self::unsol_wait_input(wait, slots, ctx, intake).await
            }
        }
    }

    fn idle_poll<C: MonotonicClock>(slots: &mut Slots, ctx: &mut SessionContext<C>) -> Self {
        // Priority order in Idle: deferred READ -> keep-alive -> unsolicited.
        // Keep-alive runs before unsolicited so an unconfirmed NULL unsolicited loop cannot starve
        // link status requests between confirm waits.
        if let Some(sol) = ctx.stage_deferred_read(&mut slots.sol) {
            return Self::stage_idle(IdleTx::Solicited(sol));
        }

        if ctx.keep_alive.is_due(ctx.clock.now()) {
            return Self::stage_idle(IdleTx::LinkStatus);
        }

        if let Some((seq, kind)) = ctx.stage_unsolicited(&mut slots.unsol) {
            return Self::stage_idle(IdleTx::Unsolicited { seq, kind });
        }

        Self::idle()
    }

    fn unsol_confirm_timeout<C: MonotonicClock>(
        wait: UnsolConfirmWait,
        slots: &mut Slots,
        ctx: &mut SessionContext<C>,
    ) -> Self {
        match wait.kind {
            UnsolicitedWaitKind::Null => {
                ctx.info.unsolicited_confirm_timeout(wait.seq, false);
                ctx.db.reset();
                ctx.state.unsolicited = UnsolicitedState::NullRequired;
                Self::idle_poll(slots, ctx)
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

                ctx.db.reset();
                ctx.state.unsolicited = UnsolicitedState::Ready {
                    retry_at: Some(ctx.new_unsolicited_retry_deadline()),
                };
                Self::idle_poll(slots, ctx)
            }
        }
    }

    async fn idle_input<C: MonotonicClock>(
        slots: &mut Slots,
        ctx: &mut SessionContext<C>,
        intake: Intake<'_>,
    ) -> Self {
        match intake {
            Intake::UnknownFunction(from, seq) => {
                // Unknown function is not a valid request, so `process_request_from_idle` is not
                // called.
                ctx.stage_unknown_function(&mut slots.transient, seq);
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
                match ctx.accept_request(&mut slots.sol, received).await {
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
                ctx.clear_written_events().await;

                if wait.series.fin {
                    return Self::idle();
                }

                // Format the next response in the multi-fragment series.
                let mut next_seq = wait.series.ecsn;
                next_seq.increment();
                let sol = ctx.next_read_fragment(&mut slots.sol, from, next_seq);
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
                ctx.db.reset();
                Self::idle_input(slots, ctx, intake).await
            }
        }
    }

    async fn unsol_wait_input<C: MonotonicClock>(
        mut wait: UnsolConfirmWait,
        slots: &mut Slots,
        ctx: &mut SessionContext<C>,
        intake: Intake<'_>,
    ) -> Self {
        match intake {
            Intake::UnknownFunction(from, seq) => {
                // Like a new request, it ends the wait for a solicited confirm. Its IIN2.0 answer
                // isn't the response to a valid request, so the outstation doesn't wait for its
                // confirm.
                wait.solicited = None;
                ctx.stage_unknown_function(&mut slots.transient, seq);
                Self::stage_unsol(wait, UnsolTx::Transient(from))
            }
            Intake::Confirm(_, header) => {
                let seq = header.control.seq;
                if header.control.uns {
                    if seq == wait.seq {
                        ctx.state.broadcast.confirmed(ResponseKind::Unsolicited);
                        ctx.info.unsolicited_confirmed(seq);
                        if matches!(wait.kind, UnsolicitedWaitKind::Events { .. }) {
                            ctx.clear_written_events().await;
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

                let answer = ctx.accept_request(&mut slots.sol, received).await;

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

    fn idle() -> Self {
        Self::Waiting(Waiting::Idle)
    }

    fn sol_wait(wait: SolConfirmWait) -> Self {
        Self::Waiting(Waiting::SolConfirmWait(wait))
    }

    fn unsol_wait(wait: UnsolConfirmWait) -> Self {
        Self::Waiting(Waiting::UnsolConfirmWait(wait))
    }

    fn stage_idle(tx: IdleTx) -> Self {
        Self::Staged(StagedTx {
            state: StagedState::Idle(tx),
        })
    }

    fn stage_sol(wait: SolConfirmWait, to: FragmentAddr) -> Self {
        Self::Staged(StagedTx {
            state: StagedState::SolWait(wait, to),
        })
    }

    fn stage_unsol(wait: UnsolConfirmWait, tx: UnsolTx) -> Self {
        Self::Staged(StagedTx {
            state: StagedState::UnsolWait(wait, tx),
        })
    }
}

enum State {
    Waiting(Waiting),
    Staged(StagedTx),
    /// Placeholder while [`Phase::input`] or [`super::core::Transmit`] owns the active phase.
    /// Observed at entry to `poll` or `input`, it means the previous operation was dropped without
    /// completing and per-connection state plus the database must be reset.
    InFlight,
}

impl From<Active> for State {
    fn from(active: Active) -> Self {
        match active {
            Active::Waiting(waiting) => Self::Waiting(waiting),
            Active::Staged(staged) => Self::Staged(staged),
        }
    }
}

/// Phase with no transmission staged: waiting on input, timers, or database updates.
enum Waiting {
    Idle,
    SolConfirmWait(SolConfirmWait),
    UnsolConfirmWait(UnsolConfirmWait),
}

impl Waiting {
    fn confirm_wait_info(&self) -> Option<ConfirmWaitInfo> {
        match *self {
            Self::Idle => None,
            Self::SolConfirmWait(wait) => Some(ConfirmWaitInfo::Solicited(wait.series.ecsn)),
            Self::UnsolConfirmWait(wait) => Some(ConfirmWaitInfo::Unsolicited(wait.seq)),
        }
    }

    /// Next timer deadline relevant while waiting.
    fn deadline<C: MonotonicClock>(&self, ctx: &SessionContext<C>) -> Option<MonotonicTime> {
        match *self {
            Self::SolConfirmWait(wait) => Some(wait.deadline),
            Self::UnsolConfirmWait(wait) => Some(wait.deadline),
            Self::Idle => {
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

#[derive(Copy, Clone, Debug)]
enum StagedState {
    Idle(IdleTx),
    SolWait(SolConfirmWait, FragmentAddr),
    UnsolWait(UnsolConfirmWait, UnsolTx),
}

impl StagedState {
    fn unstage(self) -> Waiting {
        match self {
            Self::Idle(_) => Waiting::Idle,
            Self::SolWait(wait, _) => Waiting::SolConfirmWait(wait),
            Self::UnsolWait(wait, _) => Waiting::UnsolConfirmWait(wait),
        }
    }

    fn confirm_wait_info(self) -> Option<ConfirmWaitInfo> {
        self.unstage().confirm_wait_info()
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
