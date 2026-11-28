// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::context::SessionContext;
use super::phase::{Phase, PollOutcome};
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

/// Sans-I/O DNP3 outstation application-layer protocol state machine.
pub(super) struct OutstationCore<C: MonotonicClock> {
    phase: Phase,
    slots: Slots,
    ctx: SessionContext<C>,
}

impl<C: MonotonicClock> OutstationCore<C> {
    pub(super) fn new(
        config: &OutstationConfig,
        phys_addr: PhysAddr,
        db: DatabaseHandle,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
        clock: C,
    ) -> Self {
        Self {
            phase: Phase::idle(),
            slots: Slots::new(config),
            ctx: SessionContext::new(
                config,
                phys_addr,
                db,
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
    pub(super) fn reset(&mut self) {
        self.phase.reset();
        self.slots.clear();
        self.ctx.reset();
    }

    /// Active confirm-wait tracing span metadata, if the core is waiting for a confirm.
    pub(super) fn confirm_wait_info(&self) -> Option<ConfirmWaitInfo> {
        self.phase.confirm_wait_info()
    }

    /// Drive timers and outbound staging, returning either a [`Transmit`] borrow token to write or
    /// the next timer deadline to wait on.
    pub(super) fn poll(&mut self) -> Output<'_, C> {
        match self.phase.poll(&mut self.slots, &mut self.ctx) {
            PollOutcome::Wait(deadline) => Output::Wait(deadline),
            PollOutcome::Transmit(outbound) => Output::Transmit(Transmit {
                outbound,
                phase: &mut self.phase,
                ctx: &mut self.ctx,
            }),
        }
    }

    /// Process one incoming fragment or link-layer message, staging any response to be returned by
    /// the next [`Self::poll`].
    ///
    /// Dropping this future mid-await leaves the phase in flight so the next `poll` or
    /// `handle_input` resets per-connection state and the database.
    pub(super) async fn handle_input(
        &mut self,
        input: TransportData<'_>,
        app_decode_level: AppDecodeLevel,
    ) {
        self.phase
            .input(&mut self.slots, &mut self.ctx, input, app_decode_level)
            .await;
    }
}

/// Output of [`OutstationCore::poll`].
#[must_use]
pub(super) enum Output<'a, C: MonotonicClock> {
    /// Transmit this payload, then call [`Transmit::sent`] once the write completes.
    Transmit(Transmit<'a, C>),
    /// Nothing to transmit now; wait for incoming input, database updates, or the next timer
    /// deadline.
    Wait(Option<MonotonicTime>),
}

/// Borrow token representing an outbound transmission in progress.
///
/// Holding `Transmit<'_, C>` borrows `&mut OutstationCore<C>`, statically preventing `handle_input`
/// or another `poll` while a write is underway. Call [`Self::sent`] once the write completes, or
/// tear down the session with [`OutstationCore::reset`] if the write fails or is cancelled.
/// Dropping `Transmit` without calling [`Self::sent`] leaves the transmission staged, so the next
/// `poll` offers it again.
#[must_use]
pub(super) struct Transmit<'a, C: MonotonicClock> {
    outbound: Outbound<'a>,
    phase: &'a mut Phase,
    ctx: &'a mut SessionContext<C>,
}

impl<'a, C: MonotonicClock> Transmit<'a, C> {
    /// The payload to write to the transport.
    pub(super) fn outbound(&self) -> Outbound<'a> {
        self.outbound
    }

    /// Commit the state transition after the write completes.
    pub(super) fn sent(self) {
        self.phase.sent(self.ctx);
    }
}

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
