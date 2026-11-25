// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::future::Future;
use std::ops::ControlFlow;
use std::pin::{pin, Pin};

use tracing::Instrument;

use super::core::{Outbound, Output, OutstationCore, Transmit};
use super::state::ConfirmWaitInfo;
use crate::decode::{AppDecodeLevel, DecodeLevel};
use crate::link::error::LinkError;
use crate::link::reader::LinkModes;
use crate::link::{EndpointAddress, LinkRole};
use crate::outstation::config::*;
use crate::outstation::database::DatabaseHandle;
use crate::outstation::traits::{ControlHandler, OutstationApplication, OutstationInformation};
use crate::outstation::OutstationHandle;
use crate::phys::{PhysAddr, PhysLayer};
use crate::transport::{RealTransport, Transport, TransportData, TransportRead, TransportWriter};
use crate::util::channel::Receiver;
use crate::util::clock::{MonotonicTime, TokioClock};
use crate::util::lifecycle::{Enabled, RunError, StopReason};

/// Thin Tokio driver wiring [`OutstationCore`] and a [`Transport`] to a [`PhysLayer`] and
/// [`DatabaseHandle`].
pub(crate) struct OutstationTask<T: Transport = RealTransport> {
    core: OutstationCore<TokioClock>,
    driver: Driver<T>,
}

impl OutstationTask {
    /// Create an `OutstationTask` and return it along with an `OutstationHandle` for controlling it.
    pub(crate) fn create(
        initial_state: Enabled,
        link_modes: LinkModes,
        config: OutstationConfig,
        phys_addr: PhysAddr,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
    ) -> (Self, OutstationHandle) {
        Self::with_transport(
            initial_state,
            link_modes,
            config,
            phys_addr,
            application,
            information,
            control_handler,
        )
    }
}

impl<T: Transport> OutstationTask<T> {
    /// Create an `OutstationTask` over a transport and return it along with an `OutstationHandle`
    /// for controlling it.
    pub(crate) fn with_transport(
        initial_state: Enabled,
        link_modes: LinkModes,
        config: OutstationConfig,
        phys_addr: PhysAddr,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
    ) -> (Self, OutstationHandle) {
        let (tx, rx) = crate::util::channel::request_channel();
        let handle = DatabaseHandle::new(
            config.read_header_limit(),
            config.class_zero,
            config.event_buffer_config,
        );
        let (reader, writer) = T::new(
            link_modes,
            LinkRole::Outstation {
                self_address: config.features.self_address == Feature::Enabled,
            },
            config.outstation_address,
            config.rx_buffer_size.value(),
        );
        let decode_level = config.decode_level;
        let clock = TokioClock::new();
        let core = OutstationCore::new(
            &config,
            phys_addr,
            handle.clone(),
            application,
            information,
            control_handler,
            clock,
        );
        let task = Self {
            core,
            driver: Driver {
                clock,
                reader,
                writer: TransportWriter::new(writer),
                database: handle.clone(),
                control: TaskControl {
                    messages: rx,
                    enabled: initial_state,
                    decode_level,
                },
            },
        };
        (
            task,
            OutstationHandle {
                database: handle,
                sender: tx,
            },
        )
    }

    pub(crate) fn enabled(&self) -> Enabled {
        self.driver.control.enabled
    }

    pub(crate) fn change_master_address(&mut self, address: EndpointAddress) {
        self.core.change_master_address(address);
    }

    /// Run the outstation task asynchronously until a `RunError` occurs.
    ///
    /// Cancel-safe: per-connection state is re-armed by `DropGuard` on teardown, so
    /// dropping this future tears down exactly as returning from it does.
    pub(crate) async fn run(&mut self, io: &mut PhysLayer) -> RunError {
        let Err(err) = self.run_until(io, std::future::pending::<!>()).await;
        err
    }

    /// Run the outstation task until either a `RunError` occurs or `cancel` resolves while no
    /// `handle_input` call is in flight.
    ///
    /// Writes and idle waits stay cancellable by `cancel` and stop messages, while an in-flight
    /// `handle_input` always finishes before the session switches connections or shuts down.
    pub(crate) async fn run_until<F, R>(
        &mut self,
        io: &mut PhysLayer,
        cancel: F,
    ) -> Result<R, RunError>
    where
        F: Future<Output = R>,
    {
        let mut guard = std::mem::DropGuard::new(self, Self::reset);
        let cancel = pin!(cancel);
        guard.run_session(io, cancel).await
    }

    /// Process received outstation messages while idle without a session.
    pub(crate) async fn process_next_message(&mut self) -> Result<(), StopReason> {
        self.driver.control.next_message().await
    }

    async fn run_session<F, R>(
        &mut self,
        io: &mut PhysLayer,
        mut cancel: Pin<&mut F>,
    ) -> Result<R, RunError>
    where
        F: Future<Output = R>,
    {
        loop {
            let span = self.confirm_wait_span();
            if let ControlFlow::Break(val) =
                self.run_step(io, cancel.as_mut()).instrument(span).await?
            {
                return Ok(val);
            }
        }
    }

    fn confirm_wait_span(&self) -> tracing::Span {
        match self.core.confirm_wait_info() {
            Some(ConfirmWaitInfo::Solicited(ecsn)) => {
                tracing::info_span!("SolConfirmWait", "ecsn" = ecsn.value())
            }
            Some(ConfirmWaitInfo::Unsolicited(seq)) => {
                tracing::info_span!("UnsolConfirmWait", "seq" = seq.value())
            }
            None => tracing::Span::none(),
        }
    }

    async fn run_step<F, R>(
        &mut self,
        io: &mut PhysLayer,
        cancel: Pin<&mut F>,
    ) -> Result<ControlFlow<R>, RunError>
    where
        F: Future<Output = R>,
    {
        match self.core.poll() {
            Output::Transmit(tx) => or_cancel(cancel, self.driver.transmit(io, tx)).await,
            Output::Wait {
                deadline,
                watch_database,
            } => match or_cancel(cancel, self.driver.wait(io, deadline, watch_database)).await? {
                ControlFlow::Break(val) => Ok(ControlFlow::Break(val)),
                ControlFlow::Continue(Some((input, app_level))) => {
                    // In-flight `handle_input` runs to completion before switching TCP
                    // connections or shutting down.
                    self.core.handle_input(input, app_level).await;
                    Ok(ControlFlow::Continue(()))
                }
                ControlFlow::Continue(None) => Ok(ControlFlow::Continue(())),
            },
        }
    }

    fn reset(&mut self) {
        // Re-arm, don't clear: unconfirmed events return to Unselected for re-delivery.
        self.core.reset();
        self.driver.reset();
    }
}

#[cfg(test)]
impl OutstationTask<crate::transport::mock::MockTransport> {
    pub(crate) fn set_rx_frame_info(&mut self, info: crate::link::header::FrameInfo) {
        self.driver.reader.set_rx_frame_info(info);
    }

    pub(crate) fn rx_frame_info(&self) -> crate::transport::mock::reader::RxFrameInfo {
        self.driver.reader.rx_frame_info()
    }
}

pub(crate) enum OutstationMessage {
    Enable,
    Disable,
    Shutdown,
    Configuration(ConfigurationChange),
}

pub(crate) enum ConfigurationChange {
    SetDecodeLevel(DecodeLevel),
}

impl From<ConfigurationChange> for OutstationMessage {
    fn from(x: ConfigurationChange) -> Self {
        OutstationMessage::Configuration(x)
    }
}

/// Tokio I/O, clock, database notifications, and handle control owned separately from
/// [`OutstationCore`] so transmit and wait steps can run while borrows into the core or reader are
/// live.
struct Driver<T: Transport> {
    clock: TokioClock,
    reader: T::Reader,
    writer: TransportWriter<T>,
    database: DatabaseHandle,
    control: TaskControl,
}

impl<T: Transport> Driver<T> {
    async fn transmit(
        &mut self,
        io: &mut PhysLayer,
        tx: Transmit<'_, TokioClock>,
    ) -> Result<(), RunError> {
        let level = self.control.decode_level;
        let write_result = tokio::select! {
            biased;
            stop = self.control.wait_for_stop() => return Err(stop.into()),
            res = write_outbound(&mut self.writer, io, level, tx.outbound()) => res,
        };

        write_result?;
        tx.sent();
        Ok(())
    }

    async fn wait(
        &mut self,
        io: &mut PhysLayer,
        deadline: Option<MonotonicTime>,
        watch_database: bool,
    ) -> Result<Option<(TransportData<'_>, AppDecodeLevel)>, RunError> {
        let level = self.control.decode_level;
        let sleep_target = deadline.map(|d| self.clock.to_tokio_instant(d));

        tokio::select! {
            biased;
            res = self.control.next_message() => {
                res?;
                Ok(None)
            }
            _ = sleep_option(sleep_target) => Ok(None),
            res = self.reader.read(io, level) => {
                res?;
                Ok(self
                    .reader
                    .pop()
                    .map(|data| (data, self.control.decode_level.application)))
            }
            _ = self.database.wait_for_change(), if watch_database => Ok(None),
        }
    }

    fn reset(&mut self) {
        self.reader.reset();
        self.writer.reset();
    }
}

/// Handle-driven control state kept separate from I/O so `tokio::select!` can borrow it alongside
/// reads and writes.
struct TaskControl {
    messages: Receiver<OutstationMessage>,
    enabled: Enabled,
    decode_level: DecodeLevel,
}

impl TaskControl {
    async fn next_message(&mut self) -> Result<(), StopReason> {
        match self.messages.receive().await? {
            OutstationMessage::Shutdown => Err(StopReason::Shutdown),
            OutstationMessage::Enable => {
                tracing::info!("enable communication");
                self.enabled = Enabled::Yes;
                Ok(())
            }
            OutstationMessage::Disable => {
                tracing::info!("disable communication");
                self.enabled = Enabled::No;
                Err(StopReason::Disable)
            }
            OutstationMessage::Configuration(ConfigurationChange::SetDecodeLevel(level)) => {
                tracing::info!("decode level changed to: {:?}", level);
                self.decode_level = level;
                Ok(())
            }
        }
    }

    async fn wait_for_stop(&mut self) -> StopReason {
        loop {
            if let Err(stop) = self.next_message().await {
                return stop;
            }
        }
    }
}

/// Race `fut` against `cancel`, polling `cancel` first.
///
/// `fut` must commit any post-I/O side effects (such as `Transmit::sent` or `reader.pop`) without
/// another `.await`, so `cancel` cannot drop it after I/O completes and before the commit.
async fn or_cancel<F, Fut, R, T>(
    cancel: Pin<&mut F>,
    fut: Fut,
) -> Result<ControlFlow<R, T>, RunError>
where
    F: Future<Output = R>,
    Fut: Future<Output = Result<T, RunError>>,
{
    tokio::select! {
        biased;
        val = cancel => Ok(ControlFlow::Break(val)),
        res = fut => Ok(ControlFlow::Continue(res?)),
    }
}

async fn write_outbound<T: Transport>(
    writer: &mut TransportWriter<T>,
    io: &mut PhysLayer,
    level: DecodeLevel,
    outbound: Outbound<'_>,
) -> Result<(), LinkError> {
    match outbound {
        Outbound::Fragment { destination, bytes } => {
            writer.write(io, level, destination, bytes).await
        }
        Outbound::LinkStatusRequest { destination } => {
            writer
                .send_link_status_request(io, level, destination)
                .await
        }
    }
}

async fn sleep_option(target: Option<tokio::time::Instant>) {
    match target {
        Some(target) => tokio::time::sleep_until(target).await,
        None => std::future::pending().await,
    }
}
