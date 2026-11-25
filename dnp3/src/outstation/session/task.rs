// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::future::Future;
use std::pin::{pin, Pin};

use tracing::Instrument;

use crate::decode::DecodeLevel;
use crate::link::reader::LinkModes;
use crate::link::{EndpointAddress, LinkRole};
use crate::outstation::config::*;
use crate::outstation::database::DatabaseHandle;
use crate::outstation::session::state::ConfirmWaitInfo;
use crate::outstation::session::{Outbound, Output, OutstationCore};
use crate::outstation::traits::{ControlHandler, OutstationApplication, OutstationInformation};
use crate::outstation::OutstationHandle;
use crate::phys::{PhysAddr, PhysLayer};
use crate::transport::{RealTransport, Transport, TransportRead, TransportWriter};
use crate::util::channel::Receiver;
use crate::util::clock::TokioClock;
use crate::util::lifecycle::{Enabled, RunError, StopReason};

/// Thin Tokio driver wiring [`OutstationCore`] and a [`Transport`] to a [`PhysLayer`] and
/// [`DatabaseHandle`].
pub(crate) struct OutstationTask<T: Transport = RealTransport> {
    core: OutstationCore<TokioClock>,
    clock: TokioClock,
    reader: T::Reader,
    writer: TransportWriter<T>,
    database: DatabaseHandle,
    enabled: Enabled,
    decode_level: DecodeLevel,
    messages: Receiver<OutstationMessage>,
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
            clock,
            reader,
            writer: TransportWriter::new(writer),
            database: handle.clone(),
            enabled: initial_state,
            decode_level,
            messages: rx,
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
        self.enabled
    }

    pub(crate) fn change_master_address(&mut self, address: EndpointAddress) {
        self.core.change_master_address(address);
    }

    /// Run the outstation task asynchronously until a `RunError` occurs.
    ///
    /// Cancel-safe: per-connection state is re-armed by `SessionTeardown`'s `Drop`, so
    /// dropping this future tears down exactly as returning from it does.
    pub(crate) async fn run(&mut self, io: &mut PhysLayer) -> RunError {
        match self
            .run_until(io, std::future::pending::<std::convert::Infallible>())
            .await
        {
            Ok(never) => match never {},
            Err(err) => err,
        }
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
        let guard = SessionTeardown { task: self };
        let task = &mut *guard.task; // reborrow so Drop can reset after the run future releases its borrows
        let cancel = pin!(cancel);
        task.run_session(io, cancel).await
    }

    /// Process received outstation messages while idle without a session.
    pub(crate) async fn process_next_message(&mut self) -> Result<(), StopReason> {
        handle_one_message(
            &mut self.messages,
            &mut self.enabled,
            &mut self.decode_level,
        )
        .await
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
            match self.run_step(io, cancel.as_mut()).instrument(span).await {
                StepOutcome::Continue => {}
                StepOutcome::Cancelled(val) => return Ok(val),
                StepOutcome::Stopped(err) => return Err(err),
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
        mut cancel: Pin<&mut F>,
    ) -> StepOutcome<R>
    where
        F: Future<Output = R>,
    {
        match self.core.poll() {
            Output::Transmit(tx) => {
                let level = self.decode_level;
                let outbound = tx.outbound();
                let write = async {
                    match outbound {
                        Outbound::Fragment { destination, bytes } => {
                            self.writer.write(io, level, destination, bytes).await
                        }
                        Outbound::LinkStatusRequest { destination } => {
                            self.writer
                                .send_link_status_request(io, level, destination)
                                .await
                        }
                    }
                };

                let write_result = tokio::select! {
                    biased;
                    val = cancel.as_mut() => return StepOutcome::Cancelled(val),
                    stop = wait_for_stop(&mut self.messages, &mut self.enabled, &mut self.decode_level) => {
                        return StepOutcome::Stopped(stop.into());
                    }
                    res = write => res,
                };

                match write_result {
                    Ok(()) => {
                        tx.sent();
                        StepOutcome::Continue
                    }
                    Err(err) => StepOutcome::Stopped(err.into()),
                }
            }
            Output::Wait {
                deadline,
                watch_database,
            } => {
                let level = self.decode_level;
                let sleep_target = deadline.map(|d| self.clock.to_tokio_instant(d));

                let read_ready = tokio::select! {
                    biased;
                    val = cancel.as_mut() => return StepOutcome::Cancelled(val),
                    res = handle_one_message(&mut self.messages, &mut self.enabled, &mut self.decode_level) => match res {
                        Ok(()) => false,
                        Err(stop) => return StepOutcome::Stopped(stop.into()),
                    },
                    _ = sleep_option(sleep_target) => false,
                    res = self.reader.read(io, level) => match res {
                        Ok(()) => true,
                        Err(err) => return StepOutcome::Stopped(err.into()),
                    },
                    _ = wait_for_db(&self.database, watch_database) => false,
                };

                if read_ready {
                    if let Some(input) = self.reader.pop() {
                        // In-flight `handle_input` runs to completion before switching TCP
                        // connections or shutting down.
                        self.core
                            .handle_input(input, self.decode_level.application)
                            .await;
                    }
                }

                StepOutcome::Continue
            }
        }
    }
}

#[cfg(test)]
impl OutstationTask<crate::transport::mock::MockTransport> {
    pub(crate) fn set_rx_frame_info(&mut self, info: crate::link::header::FrameInfo) {
        self.reader.set_rx_frame_info(info);
    }

    pub(crate) fn rx_frame_info(&self) -> crate::transport::mock::reader::RxFrameInfo {
        self.reader.rx_frame_info()
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

/// RAII guard that re-arms per-connection state on session teardown.
struct SessionTeardown<'a, T: Transport> {
    task: &'a mut OutstationTask<T>,
}

impl<T: Transport> Drop for SessionTeardown<'_, T> {
    fn drop(&mut self) {
        // Re-arm, don't clear: unconfirmed events return to Unselected for re-delivery.
        self.task.core.reset();
        self.task.reader.reset();
        self.task.writer.reset();
    }
}

enum StepOutcome<R> {
    Continue,
    Cancelled(R),
    Stopped(RunError),
}

async fn sleep_option(target: Option<tokio::time::Instant>) {
    match target {
        Some(target) => tokio::time::sleep_until(target).await,
        None => std::future::pending().await,
    }
}

async fn wait_for_db(database: &DatabaseHandle, enabled: bool) {
    if enabled {
        database.wait_for_change().await
    } else {
        std::future::pending().await
    }
}

async fn wait_for_stop(
    messages: &mut Receiver<OutstationMessage>,
    enabled: &mut Enabled,
    decode_level: &mut DecodeLevel,
) -> StopReason {
    loop {
        if let Err(stop) = handle_one_message(messages, enabled, decode_level).await {
            return stop;
        }
    }
}

async fn handle_one_message(
    messages: &mut Receiver<OutstationMessage>,
    enabled: &mut Enabled,
    decode_level: &mut DecodeLevel,
) -> Result<(), StopReason> {
    match messages.receive().await? {
        OutstationMessage::Shutdown => Err(StopReason::Shutdown),
        OutstationMessage::Enable => {
            tracing::info!("enable communication");
            *enabled = Enabled::Yes;
            Ok(())
        }
        OutstationMessage::Disable => {
            tracing::info!("disable communication");
            *enabled = Enabled::No;
            Err(StopReason::Disable)
        }
        OutstationMessage::Configuration(ConfigurationChange::SetDecodeLevel(level)) => {
            tracing::info!("decode level changed to: {:?}", level);
            *decode_level = level;
            Ok(())
        }
    }
}
