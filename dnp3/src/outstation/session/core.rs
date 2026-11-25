// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::context::SessionContext;
use super::phase::{Phase, StagedTx, Waiting};
use super::response::Slots;
use super::state::ConfirmWaitInfo;
use crate::decode::AppDecodeLevel;
use crate::link::EndpointAddress;
use crate::outstation::config::OutstationConfig;
use crate::outstation::database::DatabaseHandle;
use crate::outstation::traits::{ControlHandler, OutstationApplication, OutstationInformation};
use crate::phys::PhysAddr;
use crate::transport::{FragmentAddr, TransportData};
use crate::util::clock::{MonotonicClock, MonotonicTime};

/// What [`OutstationCore::poll`] wants the driver to transmit.
#[derive(Copy, Clone, Debug)]
pub(super) enum Outbound<'a> {
    /// An application-layer fragment to segment and write to `destination`.
    Fragment {
        destination: FragmentAddr,
        bytes: &'a [u8],
    },
    /// A link-layer `REQUEST_LINK_STATUS` keep-alive frame to write to `destination`.
    LinkStatusRequest { destination: FragmentAddr },
}

/// Output of [`OutstationCore::poll`].
pub(super) enum Output<'a, C: MonotonicClock> {
    /// Transmit this payload, then call [`Transmit::sent`] once the write completes.
    Transmit(Transmit<'a, C>),
    /// Nothing to transmit now; wait for incoming input, database updates (when `watch_database`
    /// is true), or `deadline`.
    Wait {
        deadline: Option<MonotonicTime>,
        watch_database: bool,
    },
}

/// Borrow token representing an outbound transmission in progress.
///
/// Holding `Transmit<'_, C>` borrows `&mut OutstationCore<C>`, statically preventing `handle_input`
/// or another `poll` while a write is underway. Dropping `Transmit` without calling [`Self::sent`]
/// leaves the transmission staged so a subsequent [`OutstationCore::poll`] offers the same payload
/// again (or a subsequent [`OutstationCore::handle_input`] discards it); session teardown calls
/// [`OutstationCore::reset`] to clear it.
pub(super) struct Transmit<'a, C: MonotonicClock> {
    outbound: Outbound<'a>,
    staged: StagedTx,
    phase: &'a mut Option<Phase>,
    ctx: &'a mut SessionContext<C>,
}

impl<'a, C: MonotonicClock> Transmit<'a, C> {
    /// The payload to write to the transport.
    pub(super) fn outbound(&self) -> Outbound<'a> {
        self.outbound
    }

    /// Commit the state transition after the write completes.
    pub(super) fn sent(self) {
        *self.phase = Some(Phase::Waiting(self.staged.on_sent(self.ctx)));
    }
}

/// Sans-I/O DNP3 outstation application-layer protocol state machine.
pub(super) struct OutstationCore<C: MonotonicClock> {
    /// Active phase, or `None` while [`Self::handle_input`] is in flight. If a caller drops
    /// `handle_input` mid-await, `phase` remains `None` and the next `poll` or `handle_input`
    /// resets per-connection state and the database so half-processed state is never observed.
    phase: Option<Phase>,
    slots: Slots,
    ctx: SessionContext<C>,
}

impl<C: MonotonicClock> OutstationCore<C> {
    pub(super) fn new(
        config: &OutstationConfig,
        phys_addr: PhysAddr,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
        clock: C,
    ) -> Self {
        Self {
            phase: Some(Phase::idle()),
            slots: Slots::new(config),
            ctx: SessionContext::new(
                config,
                phys_addr,
                application,
                information,
                control_handler,
                clock,
            ),
        }
    }

    pub(super) fn change_master_address(&mut self, address: EndpointAddress) {
        self.ctx.destination.link = address;
    }

    /// Reset per-connection protocol state and database selection between communication sessions.
    pub(super) fn reset(&mut self, db: &mut DatabaseHandle) {
        self.phase = Some(Phase::idle());
        self.slots.clear();
        self.ctx.reset(db);
    }

    fn take_phase_or_reset(&mut self, db: &mut DatabaseHandle) -> Phase {
        match self.phase.take() {
            Some(phase) => phase,
            None => {
                self.slots.clear();
                self.ctx.reset(db);
                Phase::idle()
            }
        }
    }

    /// Active confirm-wait tracing span metadata, if the core is waiting for a confirm.
    pub(super) fn confirm_wait_info(&self) -> Option<ConfirmWaitInfo> {
        self.phase.as_ref().and_then(Phase::confirm_wait_info)
    }

    /// Drive timers and outbound staging, returning either a [`Transmit`] borrow token to write or
    /// the next deadline and database-watch flag to sleep on.
    pub(super) fn poll(&mut self, db: &mut DatabaseHandle) -> Output<'_, C> {
        let phase = self.take_phase_or_reset(db);
        match phase.on_poll(&mut self.slots, &mut self.ctx, db) {
            Phase::Waiting(waiting) => wait(&mut self.phase, &self.ctx, waiting),
            Phase::Staged(staged) => {
                let Some(outbound) = staged.outbound(&self.slots, self.ctx.destination) else {
                    tracing::error!("staged response missing from slot");
                    return wait(&mut self.phase, &self.ctx, staged.unstage());
                };
                self.phase = Some(Phase::Staged(staged));
                Output::Transmit(Transmit {
                    outbound,
                    staged,
                    phase: &mut self.phase,
                    ctx: &mut self.ctx,
                })
            }
        }
    }

    /// Process one incoming fragment or link-layer message, staging any response to be returned by
    /// the next [`Self::poll`].
    ///
    /// While this future is in flight, `phase` is `None`; dropping this future mid-await leaves
    /// `phase` as `None` so the next `poll` or `handle_input` resets per-connection state and the
    /// database.
    pub(super) async fn handle_input(
        &mut self,
        input: TransportData<'_>,
        app_decode_level: AppDecodeLevel,
        db: &mut DatabaseHandle,
    ) {
        let phase = self.take_phase_or_reset(db);
        let next = match self.ctx.receive(input, app_decode_level) {
            Some(intake) => {
                phase
                    .on_input(&mut self.slots, &mut self.ctx, intake, db)
                    .await
            }
            None => phase,
        };
        self.phase = Some(next);
    }
}

fn wait<'a, C: MonotonicClock>(
    phase: &mut Option<Phase>,
    ctx: &SessionContext<C>,
    waiting: Waiting,
) -> Output<'a, C> {
    let deadline = waiting.deadline(ctx);
    let watch_database = waiting.watches_database(ctx);
    *phase = Some(Phase::Waiting(waiting));
    Output::Wait {
        deadline,
        watch_database,
    }
}
